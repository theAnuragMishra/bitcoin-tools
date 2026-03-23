use std::collections::HashSet;

use crate::forensics::types::HeuristicResult;

use super::helpers::confidence_label_from_score;
use super::{TxContext, TxSharedData};

/// Address reuse detection with reuse type classification and privacy scoring.
///
/// Address reuse types:
/// - **self_reuse**: Same address appears in both inputs and outputs of the same tx.
///   This is the strongest signal — it definitively links the output to the sender.
///   (Meiklejohn et al. 2013)
/// - **cross_tx_reuse**: Address appears in multiple transactions within the same block.
///   This may indicate the same entity is active in multiple transactions.
/// - **mixed**: Both self-reuse and cross-tx reuse detected.
///
/// Privacy impact:
/// - Self-reuse is a severe privacy leak: it reveals which output is change and
///   links the sender's identity across transaction boundaries.
/// - Cross-tx reuse within a block links transactions to the same entity.
///
/// Limitations:
/// - Cross-tx reuse detection is limited to the current block's scope.
/// - Does not detect reuse across blocks (would require a UTXO index).
/// - Some reuse may be intentional (e.g., donation addresses, exchange hot wallets).
pub fn detect_address_reuse(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    let input_set: HashSet<&str> = shared
        .input_fingerprints
        .iter()
        .map(|s| s.as_str())
        .collect();

    let output_set: HashSet<&str> = shared
        .output_fingerprints
        .iter()
        .map(|s| s.as_str())
        .collect();

    // --- Self reuse (intersection of inputs and outputs) ---
    let self_reused: HashSet<String> = input_set
        .intersection(&output_set)
        .map(|s| (*s).to_string())
        .collect();

    // --- Cross-transaction reuse ---
    let mut cross_tx_reused: HashSet<String> = HashSet::new();

    for fp in input_set.iter().chain(output_set.iter()) {
        if ctx.seen_fingerprints.contains(*fp) && !self_reused.contains(*fp) {
            cross_tx_reused.insert((*fp).to_string());
        }
    }

    let total_reused = self_reused.len() + cross_tx_reused.len();

    if total_reused == 0 {
        return HeuristicResult::not_detected();
    }

    // Weak signal guard
    if self_reused.is_empty() && cross_tx_reused.len() < 2 {
        return HeuristicResult::not_detected();
    }

    // Classify reuse type
    let reuse_type = match (self_reused.is_empty(), cross_tx_reused.is_empty()) {
        (false, false) => "mixed",
        (false, true) => "self_reuse",
        (true, false) => "cross_tx_reuse",
        (true, true) => unreachable!(),
    };

    // Numeric confidence score
    let conf_score = if !self_reused.is_empty() {
        (0.82 + (self_reused.len() as f64 * 0.04)).min(0.98)
    } else {
        (0.32 + (cross_tx_reused.len() as f64 * 0.12)).min(0.72)
    };

    // Privacy score: lower is worse privacy
    // Self-reuse devastates privacy; cross-tx reuse is also bad
    let privacy_score = if !self_reused.is_empty() {
        let penalty = (self_reused.len() as u8).saturating_mul(20);
        100u8.saturating_sub(penalty).max(5)
    } else {
        let penalty = (cross_tx_reused.len() as u8).saturating_mul(15);
        100u8.saturating_sub(penalty).max(15)
    };

    // --- Output/input overlap count ---
    let output_input_overlap = output_set.intersection(&input_set).count();

    // --- Combine reused addresses ---
    let all_reused: HashSet<String> = self_reused.union(&cross_tx_reused).cloned().collect();

    let mut result = HeuristicResult::detected();

    result.confidence = Some(confidence_label_from_score(conf_score).to_string());
    result.confidence_score = Some((conf_score * 100.0).round() / 100.0);
    result.reuse_type = Some(reuse_type.to_string());
    result.privacy_score = Some(privacy_score);

    result.reused_addresses = Some(all_reused.into_iter().take(5).collect());

    // --- Detail message ---
    let mut detail_parts = Vec::new();

    if !self_reused.is_empty() {
        detail_parts.push(format!(
            "{} address fingerprints are reused within this transaction (input to output)",
            self_reused.len()
        ));
    }

    if !cross_tx_reused.is_empty() {
        detail_parts.push(format!(
            "{} address fingerprints were seen in earlier transactions in this block",
            cross_tx_reused.len()
        ));
    }

    if output_input_overlap > 0 {
        detail_parts.push(format!(
            "{} output(s) return to an input fingerprint, consistent with change",
            output_input_overlap
        ));
    }

    result.details = Some(detail_parts.join(". "));

    result
}
