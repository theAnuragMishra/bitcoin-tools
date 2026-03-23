use crate::forensics::types::{HeuristicResult, RoundOutput};

use super::helpers::confidence_label_from_score;
use super::{TxContext, TxSharedData};

/// Round-number payment detection with denomination tier analysis.
///
/// Round-number outputs strongly suggest human-chosen payment amounts, as opposed to
/// change outputs which are computed algorithmically and tend to have "messy" values.
///
/// Denomination tiers:
/// - **btc**: Multiples of 1 BTC (100,000,000 sats) — large, round payments
/// - **deci_btc**: Multiples of 0.1 BTC (10,000,000 sats) — common payment tier
/// - **centi_btc**: Multiples of 0.01 BTC (1,000,000 sats) — standard payment
/// - **milli_btc**: Multiples of 0.001 BTC (100,000 sats) — small payment
/// - **micro_btc**: Multiples of 0.0001 BTC (10,000 sats) — micro payment
/// - **sats**: Values that are round in satoshi denomination (multiples of 1000, 10000 sats)
///   but not round in BTC — suggests the sender thinks in sats, not BTC
/// - **dust_limit**: 546 sats (Bitcoin Core's dust limit)
///
/// The sats-vs-BTC denomination detection is useful for wallet fingerprinting:
/// wallets that denominate in sats (e.g., Lightning-native wallets) produce different
/// patterns than wallets that denominate in BTC.
///
/// Limitations:
/// - Round numbers can be coincidental (change that happens to be round)
/// - Fee-aware wallets may produce round-looking change amounts
/// - Some payment protocols use specific denominations that aren't truly "round"
pub fn detect_round_number(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    let mut round_outputs = Vec::new();
    let mut denomination_tiers: Vec<String> = Vec::new();

    for (i, output) in ctx.tx.outputs.iter().enumerate() {
        if shared.output_types[i] == crate::forensics::script::ScriptType::OpReturn {
            continue;
        }

        let tier = classify_denomination_tier(output.value_sats);
        if let Some(tier_name) = tier {
            round_outputs.push(RoundOutput {
                index: i,
                value_sats: output.value_sats,
                btc_value: output.value_sats as f64 / 100_000_000.0,
            });
            if !denomination_tiers.contains(&tier_name) {
                denomination_tiers.push(tier_name);
            }
        }
    }

    if round_outputs.is_empty() {
        return HeuristicResult::not_detected();
    }

    // Determine if sender is likely denominating in BTC or sats
    let has_btc_denominated = denomination_tiers
        .iter()
        .any(|t| t == "btc" || t == "deci_btc" || t == "centi_btc" || t == "milli_btc");
    let has_sats_denominated = denomination_tiers.iter().any(|t| t == "sats");

    // Numeric confidence: more round outputs = higher confidence that these are payments
    let total_non_opreturn = shared.non_opreturn_output_indices.len();
    let round_ratio = round_outputs.len() as f64 / total_non_opreturn.max(1) as f64;
    let conf_score = if round_outputs.len() >= 3 {
        (0.7 + round_ratio * 0.3).min(1.0)
    } else if round_outputs.len() >= 2 {
        (0.6 + round_ratio * 0.2).min(0.9)
    } else {
        (0.4 + round_ratio * 0.2).min(0.7)
    };

    let mut result = HeuristicResult::detected();
    result.confidence = Some(confidence_label_from_score(conf_score).to_string());
    result.confidence_score = Some((conf_score * 100.0).round() / 100.0);
    result.round_outputs = Some(round_outputs.clone());
    result.denomination_tiers = Some(denomination_tiers.clone());

    // Build details
    let mut detail_parts = vec![format!(
        "Detected {} round-number output(s) out of {} non-OP_RETURN outputs",
        round_outputs.len(),
        total_non_opreturn
    )];

    if has_btc_denominated && !has_sats_denominated {
        detail_parts.push("sender likely denominates in BTC".to_string());
    } else if has_sats_denominated && !has_btc_denominated {
        detail_parts
            .push("sender likely denominates in sats (Lightning/modern wallet)".to_string());
    } else if has_btc_denominated && has_sats_denominated {
        detail_parts.push("mixed BTC/sats denomination".to_string());
    }

    detail_parts.push(format!("tiers: [{}]", denomination_tiers.join(", ")));

    result.details = Some(detail_parts.join(". "));

    result
}

/// Classify a satoshi value into its denomination tier.
/// Returns None if the value is not considered "round".
fn classify_denomination_tier(sats: u64) -> Option<String> {
    if sats == 0 {
        return None;
    }

    // Check from largest to smallest tier
    if sats.is_multiple_of(100_000_000) {
        return Some("btc".to_string()); // Whole BTC
    }
    if sats.is_multiple_of(10_000_000) {
        return Some("deci_btc".to_string()); // 0.1 BTC
    }
    if sats.is_multiple_of(1_000_000) {
        return Some("centi_btc".to_string()); // 0.01 BTC
    }
    if sats.is_multiple_of(100_000) {
        return Some("milli_btc".to_string()); // 0.001 BTC
    }

    // Sats-denominated rounds (not round in BTC)
    if sats.is_multiple_of(10_000) {
        return Some("sats".to_string()); // Round in sats (10k sats)
    }

    None
}
