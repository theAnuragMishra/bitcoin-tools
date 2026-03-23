use crate::forensics::types::HeuristicResult;

use super::helpers::confidence_label_from_score;
use super::{SpendIndex, TxContext, TxSharedData};

/// Peeling chain detection with in-block chain tracing.
///
/// A peeling chain is a pattern where a large UTXO is repeatedly "peeled" — each transaction
/// takes a small payment from the large UTXO and sends the remainder back as change.
/// This creates a chain of transactions: input -> (small_payment, large_change) -> ...
///
/// Pattern (Meiklejohn et al. 2013):
///   1 input -> 2 outputs (one small, one large)
///   The large output is spent in the next transaction with the same structure.
///
/// Enhanced features:
/// 1. **Value ratio analysis**: The large:small ratio indicates how much is being "peeled"
/// 2. **In-block chain tracing**: Follow the chain across transactions within the same block
///    to measure chain length (how many consecutive peel steps occur)
/// 3. **Chain continuation detection**: If the large output is spent by another tx in the block
///    with the same 1-in/2-out structure, confidence increases significantly
/// 4. **Script type consistency**: Peeling chains typically maintain the same address type
///    for the change output across the chain
///
/// Confidence tiers:
/// - High: ratio >= chain continues in-block with matching structure
/// - Medium: ratio >= 20:1
/// - Low: ratio >= 8:1
///
/// Limitations:
/// - Only detects chains within a single block; cross-block chains require UTXO index
/// - A payment where change happens to be much larger than the payment looks identical
/// - Cannot distinguish peeling chain from normal payments without broader context
pub fn detect_peeling_chain(
    ctx: &TxContext,
    shared: &TxSharedData,
    block_transactions: &[crate::forensics::tx_parser::RawTx],
    spend_index: Option<&SpendIndex>,
) -> HeuristicResult {
    // Peeling chain pattern: 1 input -> 2 outputs where one is much smaller (payment)
    // and the other is large (change), with the large one likely being spent next

    if ctx.tx.inputs.len() != 1 || ctx.tx.outputs.len() != 2 {
        return HeuristicResult::not_detected();
    }

    // Filter out OP_RETURN
    let non_opreturn = &shared.non_opreturn_output_indices;

    if non_opreturn.len() != 2 {
        return HeuristicResult::not_detected();
    }

    let val0 = ctx.tx.outputs[non_opreturn[0]].value_sats;
    let val1 = ctx.tx.outputs[non_opreturn[1]].value_sats;

    if val0 == 0 || val1 == 0 {
        return HeuristicResult::not_detected();
    }

    // One output should be significantly smaller than the other (payment << change)
    let (small_val, large_val, _small_idx, large_idx) = if val0 < val1 {
        (val0, val1, non_opreturn[0], non_opreturn[1])
    } else {
        (val1, val0, non_opreturn[1], non_opreturn[0])
    };
    let ratio = large_val as f64 / small_val as f64;

    if ratio < 8.0 {
        return HeuristicResult::not_detected();
    }

    // === In-block chain tracing ===
    // Find if the large output is spent by another transaction in the same block
    let chain_result = trace_chain(ctx, large_idx, block_transactions, shared, spend_index);

    // === Script type consistency check ===
    // In a peeling chain, the change output typically uses the same type as the input
    let input_type = if !shared.input_types.is_empty() {
        Some(shared.input_types[0])
    } else {
        None
    };
    let change_type_matches = input_type
        .map(|it| shared.output_types[large_idx] == it)
        .unwrap_or(false);

    // Confidence score
    let mut conf_score = if chain_result.continued {
        0.80
    } else if ratio >= 20.0 || change_type_matches {
        0.55
    } else {
        0.35
    };

    conf_score += (ratio.log10() / 8.0).min(0.08);
    if chain_result.continued {
        conf_score += 0.08;
        conf_score += (chain_result.chain_length as f64 * 0.02).min(0.06);
    }
    if change_type_matches {
        conf_score += 0.06;
    }
    // Small payment being a round number is additional evidence of peeling
    if super::helpers::is_round_amount(small_val) {
        conf_score += 0.04;
    }
    conf_score = conf_score.min(1.0);

    let mut result = HeuristicResult::detected();
    result.confidence = Some(confidence_label_from_score(conf_score).to_string());
    result.confidence_score = Some((conf_score * 100.0).round() / 100.0);
    result.peel_ratio = Some((ratio * 10.0).round() / 10.0);
    result.chain_continued = Some(chain_result.continued);
    result.chain_length = Some(chain_result.chain_length);
    result.input_count = Some(1);
    result.output_count = Some(2);

    // Build detail string
    let mut detail_parts = vec![format!(
        "Peel ratio is {:.1}:1 (large output {} sats, small output {} sats)",
        ratio, large_val, small_val
    )];

    if chain_result.continued {
        detail_parts.push(format!(
            "The chain continues in-block for {} step(s)",
            chain_result.chain_length
        ));
        if let Some(last_tx_idx) = chain_result.last_tx_index {
            detail_parts.push(format!(
                "Last observed chain transaction index: {}",
                last_tx_idx
            ));
        }
    } else {
        detail_parts.push("No in-block continuation was found".to_string());
    }

    if change_type_matches {
        if let Some(it) = input_type {
            detail_parts.push(format!("change output maintains {} type", it.as_str()));
        }
    }

    result.details = Some(detail_parts.join(". "));

    result
}

/// Result of in-block chain tracing
struct ChainTraceResult {
    continued: bool,
    chain_length: usize,
    last_tx_index: Option<usize>,
}

/// Trace a peeling chain within the block by following the large output.
fn trace_chain(
    ctx: &TxContext,
    large_output_index: usize,
    block_transactions: &[crate::forensics::tx_parser::RawTx],
    _shared: &TxSharedData,
    spend_index: Option<&SpendIndex>,
) -> ChainTraceResult {
    let mut current_txid = ctx.tx.txid.clone();
    let mut current_vout = large_output_index as u32;
    let mut chain_length = 1usize; // current tx is step 1
    let mut last_tx_index = None;
    let mut continued = false;

    // Follow the chain up to a reasonable depth (prevent infinite loops)
    let max_depth = 20;

    if let Some(index) = spend_index {
        for _ in 0..max_depth {
            match index.get(&(current_txid.clone(), current_vout)) {
                Some(Some((next_txid, next_vout))) => {
                    continued = true;
                    chain_length += 1;
                    current_txid = next_txid.clone();
                    current_vout = *next_vout;
                }
                _ => {
                    break;
                }
            }
        }

        return ChainTraceResult {
            continued,
            chain_length,
            last_tx_index: None,
        };
    }

    for _ in 0..max_depth {
        // Find a transaction in the block that spends (current_txid, current_vout)
        let next = block_transactions.iter().enumerate().find(|(_, next_tx)| {
            next_tx.inputs.len() == 1
                && next_tx.inputs[0].txid == current_txid
                && next_tx.inputs[0].vout == current_vout
        });

        match next {
            Some((idx, next_tx)) => {
                // Check if this continuation also has the peeling pattern (1 in, 2 out)
                if next_tx.outputs.len() != 2 {
                    // Chain broken — but we still found a continuation
                    continued = true;
                    chain_length += 1;
                    last_tx_index = Some(idx);
                    break;
                }

                // Find which output is larger (the "change" that continues the chain)
                let v0 = next_tx.outputs[0].value_sats;
                let v1 = next_tx.outputs[1].value_sats;
                if v0 == 0 || v1 == 0 {
                    continued = true;
                    chain_length += 1;
                    last_tx_index = Some(idx);
                    break;
                }

                let next_ratio = if v0 > v1 {
                    v0 as f64 / v1 as f64
                } else {
                    v1 as f64 / v0 as f64
                };

                continued = true;
                chain_length += 1;
                last_tx_index = Some(idx);

                if next_ratio < 5.0 {
                    // No longer a clear peel pattern in the continuation
                    break;
                }

                // Continue following the larger output
                current_txid = next_tx.txid.clone();
                current_vout = if v0 > v1 { 0 } else { 1 };
            }
            None => {
                break;
            }
        }
    }

    ChainTraceResult {
        continued,
        chain_length,
        last_tx_index,
    }
}
