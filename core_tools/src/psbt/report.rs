use crate::psbt::types::{
    PrivacyReport, Report, ReportInput, ReportOutput, Warning, WatchOnlyDescriptors,
};

/// Build a success report
#[allow(clippy::too_many_arguments)]
pub fn build_report(
    network: &str,
    strategy: &str,
    selected_inputs: Vec<ReportInput>,
    outputs: Vec<ReportOutput>,
    change_index: Option<u32>,
    fee_sats: u64,
    fee_rate: f64,
    vbytes: u64,
    rbf_signaling: bool,
    locktime: u32,
    locktime_type: &str,
    psbt_base64: &str,
    // signed_psbt_base64: Option<String>,
    // final_tx_hex: Option<String>,
    watch_only_descriptors: WatchOnlyDescriptors,
    privacy: PrivacyReport,
    warnings: Vec<Warning>,
) -> Report {
    Report {
        ok: true,
        network: network.to_string(),
        strategy: strategy.to_string(),
        selected_inputs,
        outputs,
        change_index,
        fee_sats,
        fee_rate_sat_vb: fee_rate,
        vbytes,
        rbf_signaling,
        locktime,
        locktime_type: locktime_type.to_string(),
        psbt_base64: psbt_base64.to_string(),
        // signed_psbt_base64,
        // final_tx_hex,
        watch_only_descriptors,
        privacy,
        warnings,
    }
}
