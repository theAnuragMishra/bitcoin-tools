use crate::forensics::types::HeuristicResult;

use crate::forensics::script::ScriptType;

use super::helpers::confidence_label_from_score;
use super::{TxContext, TxSharedData};

/// Consolidation detection with UTXO reduction metrics and value analysis.
///
/// A consolidation transaction combines many UTXOs into fewer outputs, typically done by
/// wallet operators (exchanges, services) to reduce UTXO set size and save on future fees.
///
/// Detection signals:
/// 1. **High fan-in ratio**: Many inputs relative to outputs (primary indicator)
/// 2. **Script type homogeneity**: All inputs/outputs use the same address type
///    (indicates a single wallet consolidating its own UTXOs)
/// 3. **No round-number outputs**: Consolidations don't produce payment-like round values
/// 4. **Value analysis**: Input values are often similar (suggesting automated UTXO management)
/// 5. **Single output**: True consolidation sends everything to one address
///
/// Confidence tiers:
/// - High: ratio >= 10:1 with matching output types, or ratio >= 5:1 with single output
/// - Medium: ratio >= 5:1, or ratio >= 3:1 with all types matching
/// - Low: ratio >= 3:1
///
/// Limitations:
/// - Cannot distinguish consolidation from a single user sweeping a paper wallet
/// - Exchange consolidations may look identical to individual user consolidations
/// - A consolidation with a fee change output (2 outputs) is harder to distinguish
pub fn detect_consolidation(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    // Consolidation: many inputs -> 1-2 outputs, typically same script type
    let non_opreturn_outputs = &shared.non_opreturn_output_indices;

    if ctx.tx.inputs.len() < 5 || non_opreturn_outputs.len() > 2 {
        return HeuristicResult::not_detected();
    }

    // Check if all outputs match the dominant input type
    let dominant = shared.dominant_input_type;
    let outputs_match = non_opreturn_outputs.iter().all(|&i| {
        shared.output_types[i] == dominant || shared.output_types[i] == ScriptType::Unknown
    });

    let ratio = ctx.tx.inputs.len() as f64 / non_opreturn_outputs.len().max(1) as f64;

    if ratio < 3.0 {
        return HeuristicResult::not_detected();
    }

    // Check script type homogeneity across inputs
    let all_inputs_same_type = shared.all_inputs_same_type;

    // Require stronger ownership structure for lower fan-in ratios.
    // With the 5-input minimum, ratio >= 2.5 is guaranteed (5 inputs / 2 outputs).
    // Still require structural match for ratios below 8:1.
    let structural_match = outputs_match && all_inputs_same_type;
    if !structural_match && ratio < 8.0 {
        return HeuristicResult::not_detected();
    }

    // Total consolidated value
    let total_input_value = shared.total_input_value;
    let total_output_value = shared.total_output_value;

    // Check if any outputs are round numbers (less likely in true consolidation)
    let has_round_output = non_opreturn_outputs
        .iter()
        .any(|&i| super::helpers::is_round_amount(ctx.tx.outputs[i].value_sats));

    // Input value uniformity: coefficient of variation
    // Low CV suggests automated UTXO management (e.g., all inputs from same denomination)
    let input_cv = super::helpers::coefficient_of_variation(&shared.input_values);

    // Numeric confidence
    let mut conf_score = 0.0f64;
    conf_score += (ratio / 20.0).min(0.4); // max 0.4 from ratio
    if outputs_match {
        conf_score += 0.2;
    }
    if all_inputs_same_type {
        conf_score += 0.15;
    }
    if non_opreturn_outputs.len() == 1 {
        conf_score += 0.1;
    }
    if !has_round_output {
        conf_score += 0.05;
    }
    if let Some(cv) = input_cv {
        if cv < 1.0 {
            conf_score += 0.1; // uniform input values
        }
    }

    if ratio >= 10.0 && outputs_match {
        conf_score += 0.15;
    }
    if ratio >= 5.0 && non_opreturn_outputs.len() == 1 {
        conf_score += 0.1;
    }

    conf_score = conf_score.min(1.0);

    let mut result = HeuristicResult::detected();
    result.input_count = Some(ctx.tx.inputs.len());
    result.output_count = Some(non_opreturn_outputs.len());
    result.confidence = Some(confidence_label_from_score(conf_score).to_string());
    result.confidence_score = Some((conf_score * 100.0).round() / 100.0);
    result.fan_in_ratio = Some((ratio * 10.0).round() / 10.0);
    result.dominant_input_type = Some(dominant.as_str().to_string());

    if total_input_value > 0 {
        result.consolidated_value = Some(total_input_value);
        result.total_input_value = Some(total_input_value);
        result.total_output_value = Some(total_output_value);
    }

    // Build details
    let mut detail_parts = vec![format!(
        "Fan-in ratio is {:.1}:1 ({} inputs to {} outputs)",
        ratio,
        ctx.tx.inputs.len(),
        non_opreturn_outputs.len()
    )];
    if outputs_match {
        detail_parts.push(format!(
            "all outputs match dominant input type: {}",
            dominant.as_str()
        ));
    }
    if all_inputs_same_type {
        detail_parts.push("all inputs use same script type".to_string());
    }
    if let Some(cv) = input_cv {
        if cv < 0.5 {
            detail_parts.push(format!(
                "Input values are relatively uniform (CV={:.2}), which suggests automated UTXO management",
                cv
            ));
        }
    }
    if total_input_value > 0 {
        detail_parts.push(format!(
            "consolidated value: {:.8} BTC",
            total_input_value as f64 / 100_000_000.0
        ));
    }
    result.details = Some(detail_parts.join(". "));

    result
}
