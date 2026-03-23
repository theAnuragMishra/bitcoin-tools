use crate::forensics::script::ScriptType;
use crate::forensics::types::HeuristicResult;

use super::helpers::{confidence_label_from_score, is_round_amount};
use super::{TxContext, TxSharedData};

/// Change detection using multi-signal Bayesian-inspired scoring.
///
/// Implements the following sub-heuristics (with academic references):
/// 1. Script type matching — change output reuses the sender's address type (Meiklejohn et al. 2013)
/// 2. Round number analysis — payment outputs tend to be round denominations
/// 3. Unnecessary input heuristic (Erhardt 2018) — if removing any single input still covers
///    all outputs, the wallet likely added it because the change goes back to the sender
/// 4. Output position / BIP-69 analysis — lexicographic ordering signals wallet software
/// 5. Precision analysis — trailing zeros in satoshi values indicate human-chosen amounts
/// 6. Value-ratio analysis — change is often a different magnitude than the payment
///
/// Each signal contributes weighted evidence; final confidence is derived from combined score.
pub fn detect_change(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    if ctx.tx.outputs.len() < 2 {
        return HeuristicResult::not_detected();
    }

    // Determine dominant input script type
    let dominant_input_type = shared.dominant_input_type;
    let total_input_value = shared.total_input_value;

    // === Strategy 1: Script type matching ===
    let script_match_candidates: Vec<usize> = shared
        .output_types
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            **t == dominant_input_type && **t != ScriptType::Unknown && **t != ScriptType::OpReturn
        })
        .map(|(i, _)| i)
        .collect();

    // === Strategy 2: Round number analysis ===
    let round_indices = &shared.round_output_indices;

    // === Strategy 3: Unnecessary input heuristic (Erhardt 2018) ===
    // If total input value minus the smallest input still covers total output value,
    // the smallest input was unnecessary — suggesting the wallet optimizes for change.
    let total_output_value = shared.total_output_value;
    let has_unnecessary_input = if ctx.prevouts.len() >= 2 && total_input_value > 0 {
        let min_input = shared.input_values.iter().copied().min().unwrap_or(0);
        (total_input_value - min_input) >= total_output_value
    } else {
        false
    };

    // === Strategy 4: BIP-69 lexicographic ordering check ===
    let is_bip69_sorted = shared.is_bip69_sorted;
    let precisions = &shared.output_precisions;

    // === Scoring system: combine weighted signals ===
    let mut scores: Vec<(usize, i32, usize, String)> = Vec::new();

    for i in 0..ctx.tx.outputs.len() {
        if shared.output_types[i] == ScriptType::OpReturn {
            continue; // OP_RETURN is never change
        }

        let mut score: i32 = 0;
        let mut signals: Vec<&str> = Vec::new();

        // Signal 1: Script type match (weight: 3)
        if script_match_candidates.contains(&i) {
            score += 3;
            signals.push("script_type_match");
            if script_match_candidates.len() == 1 {
                score += 1;
                signals.push("unique_script_type_match");
            }
        }

        // Signal 2: Non-round number (weight: 2)
        // Change tends to be a non-round amount when other outputs are round
        if !is_round_amount(ctx.tx.outputs[i].value_sats) && !round_indices.is_empty() {
            score += 2;
            signals.push("non_round_amount");
        }

        // Signal 3: Only non-round output in 2-output tx (weight: 2)
        if ctx.tx.outputs.len() == 2 && round_indices.len() == 1 && !round_indices.contains(&i) {
            score += 2;
            signals.push("sole_non_round");
        }

        // Signal 4: Smaller value tiebreaker in 2-output tx (weight: 1)
        if ctx.tx.outputs.len() == 2 {
            let other_idx = 1 - i;
            if ctx.tx.outputs[i].value_sats < ctx.tx.outputs[other_idx].value_sats {
                score += 1;
                signals.push("smaller_value");
            }
        }

        // Signal 5: Lower precision (fewer trailing zeros) suggests non-human choice (weight: 1)
        // Change values tend to be "messy" numbers with low precision
        let my_precision = precisions[i];
        let other_max_precision = precisions
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i && shared.output_types[*j] != ScriptType::OpReturn)
            .map(|(_, p)| *p)
            .max()
            .unwrap_or(0);
        if my_precision < other_max_precision {
            score += 1;
            signals.push("low_precision");
        }

        // Signal 6: Unnecessary input heuristic — if present, the non-script-match output
        // is more likely the payment, so the script-match output is change (weight: 1)
        if has_unnecessary_input && script_match_candidates.contains(&i) {
            score += 1;
            signals.push("unnecessary_input");
        }

        // Signal 7: Output position — in BIP-69 sorted txs, position is less informative,
        // but in non-BIP-69 txs, change is often index 1 (weight: 1)
        if !is_bip69_sorted && i == 1 && ctx.tx.outputs.len() == 2 {
            score += 1;
            signals.push("position_heuristic");
        }

        let method = if signals.is_empty() {
            "none".to_string()
        } else {
            signals.join("+")
        };

        scores.push((i, score, signals.len(), method));
    }

    // Pick highest scoring output as likely change
    scores.sort_by(|a, b| b.1.cmp(&a.1));

    if let Some((idx, score, signal_count, method)) = scores.first() {
        if *score > 0 {
            let second_score = scores.get(1).map(|(_, s, _, _)| *s).unwrap_or(0);
            let score_margin = *score - second_score;
            let is_ambiguous = second_score > 0 && score_margin <= 1;

            // Compute numeric confidence score (0.0-1.0)
            // Maximum possible score is roughly 12 with the unique-type tie-breaker.
            let max_possible: f64 = 12.0;
            let mut raw_confidence = (*score as f64 / max_possible).min(1.0);
            raw_confidence += (*signal_count as f64 * 0.03).min(0.12);
            raw_confidence += (score_margin.max(0) as f64 * 0.04).min(0.12);
            if is_ambiguous {
                raw_confidence -= 0.12;
            }
            raw_confidence = raw_confidence.clamp(0.0, 1.0);

            let weak_or_ambiguous = *score < 2 || (is_ambiguous && *score < 5);

            let mut result = HeuristicResult::detected();
            result.likely_change_index = Some(*idx);
            result.method = Some(method.clone());
            result.confidence_score = Some((raw_confidence * 100.0).round() / 100.0);
            result.confidence = Some(confidence_label_from_score(raw_confidence).to_string());
            result.change_score = Some(*score);

            // Track secondary candidate if available
            if scores.len() >= 2 {
                let (sec_idx, sec_score, _, _) = &scores[1];
                if *sec_score > 0 {
                    result.secondary_change_index = Some(*sec_idx);
                }
            }

            // Include total values for context
            if total_input_value > 0 {
                result.total_input_value = Some(total_input_value);
            }
            result.total_output_value = Some(total_output_value);

            // Detail string
            result.details = Some(format!(
                "Output {} is the most likely change candidate with score {} (margin {}). Signals: {}.{}{}{}",
                idx,
                score,
                score_margin,
                method,
                if weak_or_ambiguous {
                    " Evidence is weak or ambiguous."
                } else {
                    ""
                },
                if has_unnecessary_input {
                    " An unnecessary input pattern is present (Erhardt 2018)."
                } else {
                    ""
                },
                if is_ambiguous {
                    " A close secondary candidate is also present."
                } else {
                    ""
                }
            ));

            return result;
        }
    }

    HeuristicResult::not_detected()
}
