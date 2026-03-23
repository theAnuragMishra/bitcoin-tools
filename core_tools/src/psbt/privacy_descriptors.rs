use std::collections::{BTreeSet, HashSet};

use crate::psbt::types::{
    PrivacyReport, ReportInput, ReportOutput, ScriptType, Utxo, WatchOnlyDescriptors,
};

pub fn build_watch_only_descriptors(
    selected_inputs: &[ReportInput],
    outputs: &[ReportOutput],
) -> WatchOnlyDescriptors {
    let mut in_set = BTreeSet::new();
    let mut out_set = BTreeSet::new();

    for input in selected_inputs {
        in_set.insert(descriptor_for(
            &input.script_pubkey_hex,
            input.address.as_deref(),
        ));
    }

    for output in outputs {
        out_set.insert(descriptor_for(
            &output.script_pubkey_hex,
            output.address.as_deref(),
        ));
    }

    WatchOnlyDescriptors {
        inputs: in_set.into_iter().collect(),
        outputs: out_set.into_iter().collect(),
    }
}

fn descriptor_for(script_pubkey_hex: &str, address: Option<&str>) -> String {
    match address {
        Some(addr) if !addr.is_empty() => format!("addr({})", addr),
        _ => format!("raw({})", script_pubkey_hex),
    }
}

pub fn build_privacy_report(selected_inputs: &[Utxo], outputs: &[ReportOutput]) -> PrivacyReport {
    let mut risk_points: u32 = 0;
    let mut factors = Vec::new();

    let input_count = selected_inputs.len();
    if input_count >= 2 {
        let pts = 15 + ((input_count as u32 - 2) * 3).min(18);
        risk_points += pts;
        factors.push(format!(
            "Multiple inputs selected ({}): stronger common-ownership linkage",
            input_count
        ));
    }

    let mut unique_prevouts = HashSet::new();
    let mut duplicate_prevout_seen = false;
    for i in selected_inputs {
        let key = (i.txid.as_str(), i.vout);
        if !unique_prevouts.insert(key) {
            duplicate_prevout_seen = true;
            break;
        }
    }
    if duplicate_prevout_seen {
        risk_points += 20;
        factors.push("Duplicate prevouts detected (strong fingerprinting risk)".to_string());
    }

    let unique_script_types: HashSet<ScriptType> =
        selected_inputs.iter().map(|i| i.script_type).collect();
    if unique_script_types.len() == 1 && input_count >= 2 {
        risk_points += 8;
        factors.push("Homogeneous input script type cluster".to_string());
    }

    let payment_outputs: Vec<&ReportOutput> = outputs.iter().filter(|o| !o.is_change).collect();
    let change_outputs: Vec<&ReportOutput> = outputs.iter().filter(|o| o.is_change).collect();

    if change_outputs.len() == 1 {
        risk_points += 12;
        factors.push("Single change output heuristic likely applies".to_string());

        let change = change_outputs[0];
        if payment_outputs
            .iter()
            .any(|p| p.script_type == change.script_type)
        {
            risk_points += 8;
            factors.push("Change script type matches payment output type".to_string());
        }

        if change.value_sats % 1000 != 0 {
            risk_points += 6;
            factors.push("Non-round change amount is easy to fingerprint".to_string());
        }
    }

    if payment_outputs.len() >= 2 {
        let mut seen_scripts = HashSet::new();
        let mut reused = false;
        for out in &payment_outputs {
            if !seen_scripts.insert(out.script_pubkey_hex.as_str()) {
                reused = true;
                break;
            }
        }
        if reused {
            risk_points += 10;
            factors.push("Repeated destination script among payments".to_string());
        }
    }

    let risk_points = risk_points.min(100);
    let score = (100 - risk_points) as u8;
    let risk_level = if risk_points >= 70 {
        "high"
    } else if risk_points >= 40 {
        "medium"
    } else {
        "low"
    }
    .to_string();

    PrivacyReport {
        score,
        risk_level,
        factors,
    }
}
