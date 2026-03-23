use crate::psbt::coin_selection;
use crate::psbt::fee;
use crate::psbt::fixture;
use crate::psbt::privacy_descriptors;
use crate::psbt::psbt_builder;
use crate::psbt::rbf_locktime;
use crate::psbt::report;
use crate::psbt::types::*;
use crate::psbt::vbytes;
use crate::psbt::warnings;

/// Main builder: takes a fixture JSON string and returns either a Report or an ErrorReport
pub fn build_from_fixture(json_str: &str) -> Result<Report, BuilderError> {
    // 1. Parse and validate the fixture
    let fixture = fixture::parse_fixture(json_str)?;

    // 2. Compute RBF/locktime parameters
    let rbf_lt =
        rbf_locktime::compute_rbf_locktime(fixture.rbf, fixture.locktime, fixture.current_height);

    // 3. Prepare output script types for vbyte estimation
    let payment_output_types: Vec<ScriptType> =
        fixture.payments.iter().map(|p| p.script_type).collect();

    let total_payments: u64 = fixture
        .payments
        .iter()
        .try_fold(0u64, |acc, p| acc.checked_add(p.value_sats))
        .ok_or_else(|| {
            BuilderError::new("INVALID_FIXTURE", "Total payment amounts overflow u64")
        })?;

    // 4. Get max_inputs from policy
    let max_inputs = fixture.policy.as_ref().and_then(|p| p.max_inputs);

    // 5. Select coins (use exact script hexes for tighter fee estimation)
    let output_script_hexes: Vec<String> = fixture
        .payments
        .iter()
        .map(|p| p.script_pubkey_hex.clone())
        .collect();

    let selection = coin_selection::select_coins_with_scripts(
        &fixture.utxos,
        total_payments,
        fixture.fee_rate_sat_vb,
        &payment_output_types,
        &fixture.change.script_type,
        max_inputs,
        Some(&output_script_hexes),
        Some(&fixture.change.script_pubkey_hex),
    )?;

    // 6. Compute fee and change
    let input_types: Vec<ScriptType> = selection.selected.iter().map(|u| u.script_type).collect();

    let fee_result = fee::compute_fee_and_change_with_scripts(
        &input_types,
        &payment_output_types,
        &fixture.change.script_type,
        selection.total_value,
        total_payments,
        fixture.fee_rate_sat_vb,
        Some(&output_script_hexes),
        Some(&fixture.change.script_pubkey_hex),
    )?;

    // Verify the fee meets the minimum
    let final_vbytes = fee_result.vbytes;
    let min_fee = (final_vbytes as f64 * fixture.fee_rate_sat_vb).ceil() as u64;

    // The fee must be at least the minimum
    if fee_result.fee_sats < min_fee {
        return Err(BuilderError::new(
            "INSUFFICIENT_FUNDS",
            &format!(
                "Fee {} is below minimum {} for {} vbytes at {} sat/vB",
                fee_result.fee_sats, min_fee, final_vbytes, fixture.fee_rate_sat_vb
            ),
        ));
    }

    // 7. Build payment script_pubkeys
    let payment_scripts: Vec<(bitcoin::ScriptBuf, u64)> = fixture
        .payments
        .iter()
        .map(|p| {
            let script = psbt_builder::parse_script_pubkey(&p.script_pubkey_hex)?;
            Ok((script, p.value_sats))
        })
        .collect::<Result<Vec<_>, BuilderError>>()?;

    // 8. Build change output if needed
    let change_output = if fee_result.has_change {
        let change_script = psbt_builder::parse_script_pubkey(&fixture.change.script_pubkey_hex)?;
        Some((change_script, fee_result.change_sats))
    } else {
        None
    };

    // 9. Build the PSBT
    let psbt = psbt_builder::build_psbt(
        &selection.selected,
        &payment_scripts,
        change_output,
        rbf_lt.n_sequence,
        rbf_lt.n_locktime,
    )?;

    let psbt_base64 = psbt_builder::psbt_to_base64(&psbt);

    // Part of stretch goals
    // let (signed_psbt_base64, final_tx_hex) = stretch::try_sign_and_finalize_with_test_keys(
    //     &psbt,
    //     &selection.selected,
    //     fixture.test_signing_wifs.as_deref(),
    // )?;

    // 10. Build report outputs
    let mut report_outputs: Vec<ReportOutput> = Vec::new();
    for (i, payment) in fixture.payments.iter().enumerate() {
        report_outputs.push(ReportOutput {
            n: i as u32,
            value_sats: payment.value_sats,
            script_pubkey_hex: payment.script_pubkey_hex.clone(),
            script_type: payment.script_type.clone(),
            address: payment.address.clone(),
            is_change: false,
        });
    }

    let change_index = if fee_result.has_change {
        let idx = report_outputs.len() as u32;
        report_outputs.push(ReportOutput {
            n: idx,
            value_sats: fee_result.change_sats,
            script_pubkey_hex: fixture.change.script_pubkey_hex.clone(),
            script_type: fixture.change.script_type.clone(),
            address: fixture.change.address.clone(),
            is_change: true,
        });
        Some(idx)
    } else {
        None
    };

    // 11. Build report inputs
    let report_inputs: Vec<ReportInput> = selection
        .selected
        .iter()
        .map(|u| ReportInput {
            txid: u.txid.clone(),
            vout: u.vout,
            value_sats: u.value_sats,
            script_pubkey_hex: u.script_pubkey_hex.clone(),
            script_type: u.script_type.clone(),
            address: u.address.clone(),
        })
        .collect();

    // 12. Compute actual fee rate for the report
    let actual_fee_rate = fee_result.fee_sats as f64 / final_vbytes as f64;

    // 13. Check for uneconomical inputs (inputs that cost more to spend than they're worth)
    let has_uneconomical_inputs = selection.selected.iter().any(|u| {
        let iw = vbytes::input_weight(&u.script_type);
        let spending_cost = (iw as f64 * fixture.fee_rate_sat_vb / 4.0).ceil() as u64;
        u.value_sats <= spending_cost
    });

    // 14. Generate warnings
    let warns = warnings::generate_warnings(
        fee_result.fee_sats,
        actual_fee_rate,
        fee_result.has_change,
        fee_result.change_sats,
        rbf_lt.rbf_signaling,
        selection.selected.len(),
        final_vbytes,
        fixture.fee_rate_sat_vb,
        has_uneconomical_inputs,
    );

    // 15. Build the report
    let descriptors =
        privacy_descriptors::build_watch_only_descriptors(&report_inputs, &report_outputs);
    let privacy = privacy_descriptors::build_privacy_report(&selection.selected, &report_outputs);

    let result_report = report::build_report(
        &fixture.network,
        selection.strategy.name(),
        report_inputs,
        report_outputs,
        change_index,
        fee_result.fee_sats,
        actual_fee_rate,
        final_vbytes,
        rbf_lt.rbf_signaling,
        rbf_lt.n_locktime,
        &rbf_lt.locktime_type,
        &psbt_base64,
        descriptors,
        privacy,
        warns,
    );

    Ok(result_report)
}
