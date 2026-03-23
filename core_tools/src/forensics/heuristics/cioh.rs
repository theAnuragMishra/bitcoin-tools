use crate::forensics::types::HeuristicResult;

use super::helpers::confidence_label_from_score;
use super::{TxContext, TxSharedData};

/// CIOH: Common Input Ownership Heuristic
///
/// The foundational chain analysis assumption: all inputs to a transaction are likely
/// controlled by the same entity. This enhanced version incorporates:
///
/// 1. **Input count analysis** with graduated confidence tiers
/// 2. **Script type diversity** — multiple script types suggest wallet migration or
///    multi-path spending, strengthening the single-entity hypothesis (a single wallet
///    that has accumulated UTXOs over address format upgrades)
/// 3. **Input value distribution** — uniform input values suggest CoinJoin (weakening CIOH);
///    varied values suggest organic wallet spending (strengthening CIOH)
/// 4. **CoinJoin exclusion** — if the transaction has CoinJoin-like characteristics
///    (many equal-value outputs), CIOH confidence is downgraded since CoinJoin deliberately
///    violates the common ownership assumption
/// 5. **Probabilistic confidence score** (0.0-1.0) computed from multiple signals
pub fn detect_cioh(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    if ctx.tx.inputs.len() <= 1 {
        return HeuristicResult::not_detected();
    }

    let num_inputs = ctx.tx.inputs.len();
    let num_outputs = ctx.tx.outputs.len();

    // --- Signal 1: Input count (base confidence) ---
    // More inputs = stronger CIOH signal, but with diminishing returns
    let input_count_score: f64 = if num_inputs >= 10 {
        0.90
    } else if num_inputs >= 5 {
        0.80
    } else if num_inputs >= 3 {
        0.70
    } else {
        0.60
    };

    // --- Signal 2: Script type analysis ---
    let num_distinct_types = shared.input_type_count;

    // A wallet that has accumulated UTXOs across format upgrades will naturally have diverse
    // input types. This STRENGTHENS CIOH in isolation. However, in a CoinJoin context
    // (diverse types from different participants), it weakens CIOH.
    let type_diversity_modifier: f64 = if num_distinct_types >= 3 {
        0.05
    } else if num_distinct_types == 2 {
        0.03
    } else {
        0.0
    };

    // --- Signal 3: CoinJoin detection cross-check ---
    let mut coinjoin_penalty: f64 = 0.0;
    let mut value_analysis_detail = String::new();

    if !ctx.prevouts.is_empty() {
        // Check for equal input values
        let value_counts = super::helpers::value_frequency_map(&shared.input_values);
        let max_equal_inputs = value_counts.values().max().copied().unwrap_or(0);
        let equal_input_ratio = max_equal_inputs as f64 / num_inputs as f64;

        // Check for equal output values
        let max_equal_outputs = shared
            .output_value_counts
            .values()
            .max()
            .copied()
            .unwrap_or(0);

        // Strong CoinJoin signal penalizes CIOH confidence heavily
        if max_equal_outputs >= 3
            && num_inputs >= 3
            && num_outputs >= 3
            && (equal_input_ratio >= 0.5 || max_equal_outputs as f64 / num_outputs as f64 >= 0.4)
        {
            coinjoin_penalty = 0.30;
            value_analysis_detail = format!(
                "CoinJoin-like structure: {} equal outputs, {:.0}% equal inputs — CIOH weakened",
                max_equal_outputs,
                equal_input_ratio * 100.0
            );
        } else if max_equal_outputs >= 2 && num_inputs >= 5 {
            coinjoin_penalty = 0.10;
            value_analysis_detail = format!(
                "Possible CoinJoin: {} equal outputs among {} inputs",
                max_equal_outputs, num_inputs
            );
        }

        // Coefficient of variation for input values — very uniform inputs are suspicious
        if let Some(cv) = super::helpers::coefficient_of_variation(&shared.input_values) {
            if cv < 0.1 && num_inputs >= 3 {
                coinjoin_penalty = coinjoin_penalty.max(0.15);
                if value_analysis_detail.is_empty() {
                    value_analysis_detail = format!(
                        "Input values highly uniform (CV={:.3}) — possible CoinJoin",
                        cv
                    );
                }
            }
        }
    }

    // --- Compute final probabilistic confidence score ---
    let raw_score = input_count_score + type_diversity_modifier - coinjoin_penalty;
    let confidence_score = raw_score.clamp(0.1, 0.95);

    let mut result = HeuristicResult::detected();
    result.input_count = Some(num_inputs);
    result.confidence = Some(confidence_label_from_score(confidence_score).to_string());
    result.confidence_score = Some((confidence_score * 1000.0).round() / 1000.0);
    result.input_type_count = Some(num_distinct_types);

    if shared.dominant_input_type != crate::forensics::script::ScriptType::Unknown {
        result.dominant_input_type = Some(shared.dominant_input_type.as_str().to_string());
    }

    let mut details_parts = Vec::new();
    details_parts.push(format!(
        "{} inputs are spent together across {} distinct script type(s)",
        num_inputs, num_distinct_types
    ));
    if !value_analysis_detail.is_empty() {
        details_parts.push(value_analysis_detail);
    }
    if coinjoin_penalty > 0.0 {
        details_parts.push(format!(
            "Confidence is reduced by {:.0}% due to CoinJoin-like indicators",
            coinjoin_penalty * 100.0
        ));
    }
    result.details = Some(details_parts.join(". "));

    if shared.total_input_value > 0 {
        result.total_input_value = Some(shared.total_input_value);
    }

    result
}
