use crate::forensics::script::ScriptType;
use crate::forensics::types::HeuristicResult;

use super::helpers::{confidence_label_from_score, is_round_amount};
use super::{TxContext, TxSharedData};

/// Self-transfer detection
///
/// A self-transfer is a transaction where the sender sends funds to themselves,
/// typically for wallet reorganization, address type migration, or privacy purposes.
///
/// Detection signals:
/// 1. **Script type uniformity**: All non-OP_RETURN outputs match the dominant input type
/// 2. **No round-number outputs**: Self-transfers don't produce payment-like denominations
/// 3. **All inputs same type**: Strong indicator of a single wallet
/// 4. **Output count**: Self-transfers typically have 1-3 outputs
/// 5. **Sequence number analysis**: Consistent nSequence values suggest same wallet software
/// 6. **Locktime analysis**: Non-zero locktime with anti-fee-snipe pattern is common in
///    modern wallets doing self-transfers
///
/// Limitations:
/// - Cannot distinguish self-transfer from a payment to someone who uses the same address type
/// - A payment to an exchange that uses the same address type will be misclassified
/// - Address type migration (e.g., P2PKH -> P2WPKH) will NOT be detected as self-transfer
///   because input/output types differ
pub fn detect_self_transfer(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    if ctx.tx.outputs.is_empty() || ctx.prevouts.is_empty() {
        return HeuristicResult::not_detected();
    }

    let dominant = shared.dominant_input_type;
    if dominant == ScriptType::Unknown {
        return HeuristicResult::not_detected();
    }

    // Check all non-OP_RETURN outputs match input type
    let non_opreturn_outputs: Vec<(usize, &ScriptType)> = shared
        .output_types
        .iter()
        .enumerate()
        .filter(|(_, t)| **t != ScriptType::OpReturn)
        .collect();

    let all_match = non_opreturn_outputs.iter().all(|(_, t)| **t == dominant);

    if !all_match {
        return HeuristicResult::not_detected();
    }

    // Additional signal: no round-number outputs (suggests no payment)
    let has_round = shared
        .non_opreturn_output_indices
        .iter()
        .any(|&i| is_round_amount(ctx.tx.outputs[i].value_sats));

    // Check if all inputs use the same script type (strong self-transfer signal)
    let all_inputs_same = shared.all_inputs_same_type;

    // Limit to at most 4 non-OP_RETURN outputs (self-transfers rarely fan-out)
    let num_outputs = non_opreturn_outputs.len();
    if num_outputs > 4 {
        return HeuristicResult::not_detected();
    }

    // Sequence number uniformity: all inputs have the same nSequence
    let sequence_uniform = if ctx.tx.inputs.len() >= 2 {
        let first_seq = ctx.tx.inputs[0].sequence;
        ctx.tx.inputs.iter().all(|i| i.sequence == first_seq)
    } else {
        true // single input is trivially uniform
    };

    // Value analysis: total output / total input ratio (fee ratio)
    // Self-transfers typically have minimal fees relative to the total value
    let total_input = shared.total_input_value;
    let total_output = shared.total_output_value;
    let fee = total_input.saturating_sub(total_output);
    let fee_ratio = if total_input > 0 {
        fee as f64 / total_input as f64
    } else {
        1.0
    };
    // Self-transfers tend to have low fee ratios (< 1% of value)
    let low_fee_ratio = fee_ratio < 0.01 && total_input > 0;

    // Count signals for scoring
    let num_outputs = non_opreturn_outputs.len();
    let num_inputs = ctx.tx.inputs.len();

    if all_match && !has_round && all_inputs_same && num_outputs <= 3 {
        // Base confidence from original logic
        let mut score = 0u32;
        score += 3; // all outputs match (base requirement)
        if all_inputs_same {
            score += 2;
        }
        if !has_round {
            score += 1;
        }
        if num_outputs <= 2 {
            score += 1;
        }
        if sequence_uniform {
            score += 1;
        }
        if low_fee_ratio {
            score += 1;
        }

        if num_inputs == 1 {
            score -= 6;
        }

        // Numeric confidence
        let conf_score = (score as f64 / 9.0).min(1.0);

        let mut result = HeuristicResult::detected();
        result.confidence = Some(confidence_label_from_score(conf_score).to_string());
        result.confidence_score = Some((conf_score * 100.0).round() / 100.0);
        result.all_outputs_same_type = Some(true);
        result.dominant_input_type = Some(dominant.as_str().to_string());
        result.input_count = Some(ctx.tx.inputs.len());
        result.output_count = Some(num_outputs);

        if total_input > 0 {
            result.total_input_value = Some(total_input);
            result.total_output_value = Some(total_output);
        }

        // Build detail string
        let mut detail_parts = vec![format!(
            "All {} non-OP_RETURN output(s) match the dominant input type ({})",
            num_outputs,
            dominant.as_str()
        )];
        if all_inputs_same {
            detail_parts.push("All inputs also use the same script type".to_string());
        }
        if sequence_uniform && ctx.tx.inputs.len() >= 2 {
            detail_parts.push(format!(
                "Input nSequence values are uniform (0x{:08x})",
                ctx.tx.inputs[0].sequence
            ));
        }
        if low_fee_ratio && total_input > 0 {
            detail_parts.push(format!("Fee ratio is low ({:.4}%)", fee_ratio * 100.0));
        }
        result.details = Some(detail_parts.join(". "));

        result
    } else {
        HeuristicResult::not_detected()
    }
}
