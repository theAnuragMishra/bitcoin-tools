use crate::forensics::types::HeuristicResult;
use std::collections::HashMap;

use super::helpers::confidence_label_from_score;
use super::{TxContext, TxSharedData};

/// CoinJoin detection with sub-protocol fingerprinting.
///
/// Detects the following CoinJoin implementations:
/// - **Wasabi Wallet (WabiSabi)**: Typically uses P2WPKH inputs, many equal-denomination outputs
///   plus coordinator fee outputs. Common denominations: 0.1, 0.01, 0.001 BTC.
/// - **Whirlpool (Samourai)**: Fixed pool sizes (0.5, 0.05, 0.01, 0.001 BTC), exactly 5 outputs
///   of equal value, 5 inputs. No toxic change in the mixing tx itself.
/// - **JoinMarket**: Variable participant counts, maker/taker model. More diverse input types,
///   outputs include both equal-value (maker) and non-equal (taker change).
/// - **Generic CoinJoin**: Falls back if no specific protocol matches.
///
/// Confidence model:
/// - High: >=5 equal outputs, ratio >= 0.5, matches a known protocol signature
/// - Medium: >=3 equal outputs with either diverse inputs or high ratio
/// - Low: >=2 equal outputs with many inputs (potential PayJoin or weak CoinJoin)
///
/// Known limitations:
/// - Cannot definitively distinguish CoinJoin from batch payments with coincidental equal values
/// - Coordinator fee detection is heuristic (smallest output after removing equal-value set)
/// - Whirlpool detection is pattern-based; non-standard pool sizes will miss
pub fn detect_coinjoin(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    if ctx.tx.inputs.len() < 3 || ctx.tx.outputs.len() < 3 {
        return HeuristicResult::not_detected();
    }

    if ctx.prevouts.is_empty() {
        return HeuristicResult::not_detected();
    }

    // Count equal-value outputs (excluding OP_RETURN and zero-value)
    let output_values = &shared.output_values;

    if output_values.len() < 3 {
        return HeuristicResult::not_detected();
    }

    let non_opreturn_output_count = output_values.len();
    let dominant_input_type = shared.dominant_input_type;
    let same_type_output_count = shared
        .non_opreturn_output_indices
        .iter()
        .filter(|&&i| shared.output_types[i] == dominant_input_type)
        .count();

    let value_counts: &HashMap<u64, usize> = &shared.output_value_counts;

    // Count distinct denominations with >= 2 occurrences
    let denomination_groups: Vec<(u64, usize)> = value_counts
        .iter()
        .filter(|(_, &count)| count >= 2)
        .map(|(&val, &count)| (val, count))
        .collect();

    let denomination_count = denomination_groups.len();

    // Find the most common equal value
    let max_equal = value_counts
        .iter()
        .filter(|(_, &count)| count >= 2)
        .max_by_key(|(_, &count)| count);

    if let Some((&equal_value, &equal_count)) = max_equal {
        let input_type_diversity = { shared.input_type_count };

        let ratio = equal_count as f64 / non_opreturn_output_count as f64;
        let non_equal_output_count = non_opreturn_output_count.saturating_sub(equal_count);

        // === Sub-protocol fingerprinting ===
        let coinjoin_type = detect_coinjoin_protocol(
            ctx,
            shared,
            equal_value,
            equal_count,
            output_values,
            non_opreturn_output_count,
        );

        // === Coordinator fee detection ===
        // In many CoinJoin implementations, the coordinator takes a small fee.
        // This manifests as one or more small outputs that are not part of the equal-value set.
        let non_equal_outputs: Vec<u64> = output_values
            .iter()
            .filter(|&&v| v != equal_value)
            .copied()
            .collect();
        let possible_coordinator_fee = if !non_equal_outputs.is_empty() {
            let min_non_equal = *non_equal_outputs.iter().min().unwrap();
            // Coordinator fee is typically << equal_value
            if min_non_equal < equal_value / 10 {
                Some(min_non_equal)
            } else {
                None
            }
        } else {
            None
        };

        // === Confidence computation ===
        // Strong CoinJoin signal: many equal outputs (>=3), diverse inputs, high ratio
        let many_participants = ctx.tx.inputs.len() >= 5;
        let broad_equal_set = ratio >= 0.5;
        let batching_bias = same_type_output_count >= non_opreturn_output_count.saturating_sub(1);
        let structurally_coinjoin_like = equal_count >= 3
            && many_participants
            && (input_type_diversity >= 2 || broad_equal_set)
            && (!batching_bias || coinjoin_type != "generic")
            && (equal_count >= 4 || coinjoin_type != "generic")
            && (non_equal_output_count <= 3 || ratio >= 0.6);

        if structurally_coinjoin_like {
            // Numeric confidence score
            let mut conf_score = 0.0f64;
            conf_score += (equal_count as f64 / 10.0).min(0.4); // max 0.4 from equal count
            conf_score += ratio * 0.3; // max 0.3 from ratio
            if input_type_diversity >= 2 {
                conf_score += 0.1;
            }
            if coinjoin_type != "generic" {
                conf_score += 0.15;
            }
            if possible_coordinator_fee.is_some() {
                conf_score += 0.05;
            }
            if batching_bias && coinjoin_type == "generic" {
                conf_score -= 0.15;
            }
            conf_score = conf_score.min(1.0);
            conf_score = conf_score.max(0.0);

            let mut result = HeuristicResult::detected();
            result.equal_output_count = Some(equal_count);
            result.equal_output_value = Some(equal_value);
            result.input_count = Some(ctx.tx.inputs.len());
            result.output_count = Some(ctx.tx.outputs.len());
            result.confidence = Some(confidence_label_from_score(conf_score).to_string());
            result.confidence_score = Some((conf_score * 100.0).round() / 100.0);
            result.coinjoin_type = Some(coinjoin_type.to_string());
            result.denomination_count = Some(denomination_count);

            // Privacy score: CoinJoin increases privacy
            let privacy_boost = (equal_count as u8).saturating_mul(10).min(90);
            result.privacy_score = Some(privacy_boost);

            let mut detail_parts = vec![format!(
                "Detected {} equal-value outputs at {} sats ({:.8} BTC)",
                equal_count,
                equal_value,
                equal_value as f64 / 100_000_000.0
            )];
            if let Some(fee) = possible_coordinator_fee {
                detail_parts.push(format!("Possible coordinator-fee output: {} sats", fee));
            }
            if denomination_count > 1 {
                detail_parts.push(format!(
                    "{} distinct denomination groups are present",
                    denomination_count
                ));
            }
            detail_parts.push(format!("Equal-output ratio is {:.2}", ratio));
            detail_parts.push(format!("Input type diversity is {}", input_type_diversity));
            detail_parts.push(format!("Protocol signature: {}", coinjoin_type));
            result.details = Some(detail_parts.join(". "));

            return result;
        }

        // Weaker signal: at least 2 equal outputs with many inputs
        if equal_count >= 2
            && ctx.tx.inputs.len() >= 5
            && ratio >= 0.3
            && (input_type_diversity >= 2 || coinjoin_type == "joinmarket")
        {
            let mut result = HeuristicResult::detected();
            result.equal_output_count = Some(equal_count);
            result.equal_output_value = Some(equal_value);
            result.input_count = Some(ctx.tx.inputs.len());
            result.output_count = Some(ctx.tx.outputs.len());
            let conf_score = 0.30;
            result.confidence = Some(confidence_label_from_score(conf_score).to_string());
            result.confidence_score = Some(conf_score);
            result.coinjoin_type = Some(coinjoin_type.to_string());
            result.denomination_count = Some(denomination_count);
            result.details = Some(format!(
                "Weak CoinJoin signal with {} equal outputs at {} sats across {} inputs. Protocol signature: {}.",
                equal_count,
                equal_value,
                ctx.tx.inputs.len(),
                coinjoin_type
            ));
            return result;
        }
    }

    HeuristicResult::not_detected()
}

/// Attempt to identify the specific CoinJoin protocol based on transaction structure.
fn detect_coinjoin_protocol(
    _ctx: &TxContext,
    shared: &TxSharedData,
    equal_value: u64,
    equal_count: usize,
    output_values: &[u64],
    non_opreturn_output_count: usize,
) -> &'static str {
    // === Whirlpool detection ===
    // Whirlpool has fixed pool sizes and exactly 5 equal outputs
    let whirlpool_denominations: &[u64] = &[
        50_000_000, // 0.5 BTC
        5_000_000,  // 0.05 BTC
        1_000_000,  // 0.01 BTC
        100_000,    // 0.001 BTC
    ];
    if equal_count == 5
        && non_opreturn_output_count == 5
        && whirlpool_denominations.contains(&equal_value)
    {
        // All outputs should be equal in a Whirlpool tx (no toxic change)
        let all_equal = output_values.iter().all(|&v| v == equal_value);
        if all_equal {
            return "whirlpool";
        }
    }

    // === Wasabi Wallet detection ===
    // Wasabi uses P2WPKH predominantly, many equal outputs (often > 5),
    // with change outputs and possibly coordinator fee
    let all_inputs_segwit = shared.all_inputs_segwit;
    if all_inputs_segwit && equal_count >= 5 {
        // Wasabi often has denominations in standard tiers
        let wasabi_tiers: &[u64] = &[
            10_000_000,  // 0.1 BTC
            1_000_000,   // 0.01 BTC
            100_000,     // 0.001 BTC
            50_000_000,  // 0.5 BTC
            100_000_000, // 1 BTC
        ];
        if wasabi_tiers.contains(&equal_value) {
            return "wasabi";
        }
        // Even without a known tier, high equal count with all-segwit is Wasabi-like
        if equal_count >= 8 {
            return "wasabi";
        }
    }

    // === JoinMarket detection ===
    // JoinMarket has a maker/taker model. Makers provide inputs and receive equal-value outputs.
    // Takers pay a fee. The tx typically has diverse input types (makers use different wallets).
    if shared.input_type_count >= 3 && equal_count >= 3 {
        // JoinMarket txs tend to have non-equal outputs alongside equal ones (taker change)
        let non_equal_count = output_values.iter().filter(|&&v| v != equal_value).count();
        if (1..=3).contains(&non_equal_count) {
            return "joinmarket";
        }
    }

    "generic"
}
