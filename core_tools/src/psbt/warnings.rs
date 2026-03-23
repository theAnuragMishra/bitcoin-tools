use crate::psbt::fee::DUST_THRESHOLD;
use crate::psbt::types::Warning;

const HIGH_FEE_AMOUNT_THRESHOLD: u64 = 1_000_000;
const HIGH_FEE_RATE_THRESHOLD: f64 = 200.0;
const LOW_FEE_RATE_THRESHOLD: f64 = 1.0;
const MANY_INPUTS_THRESHOLD: usize = 50;
const LARGE_TX_VBYTES_THRESHOLD: u64 = 10_000;

/// Generate warnings based on transaction parameters.
///
/// Required warnings:
/// - HIGH_FEE: fee_sats > 1_000_000 OR fee_rate_sat_vb > 200
/// - DUST_CHANGE: a change output exists with value_sats < 546
/// - SEND_ALL: no change output was created
/// - RBF_SIGNALING: rbf_signaling is true
///
/// Additional safety warnings (we define "unsafe"):
/// - LOW_FEE_RATE: fee rate < 1 sat/vB — may not confirm in reasonable time
/// - MANY_INPUTS: more than 50 inputs — high cost, complex transaction
/// - LARGE_TRANSACTION: vbytes > 10000 — may face relay/confirmation issues
/// - UNECONOMICAL_INPUTS: some selected inputs cost more to spend than they're worth
pub fn generate_warnings(
    fee_sats: u64,
    fee_rate: f64,
    has_change: bool,
    change_sats: u64,
    rbf_signaling: bool,
    num_inputs: usize,
    vbytes: u64,
    target_fee_rate: f64,
    has_uneconomical_inputs: bool,
) -> Vec<Warning> {
    let mut warnings = Vec::new();

    // HIGH_FEE: fee_sats > 1,000,000 OR fee_rate > 200
    if fee_sats > HIGH_FEE_AMOUNT_THRESHOLD || fee_rate > HIGH_FEE_RATE_THRESHOLD {
        warnings.push(Warning {
            code: "HIGH_FEE".to_string(),
        });
    }

    // DUST_CHANGE: change output exists with value < 546
    if has_change && change_sats < DUST_THRESHOLD {
        warnings.push(Warning {
            code: "DUST_CHANGE".to_string(),
        });
    }

    // SEND_ALL: no change output
    if !has_change {
        warnings.push(Warning {
            code: "SEND_ALL".to_string(),
        });
    }

    // RBF_SIGNALING: rbf_signaling is true
    if rbf_signaling {
        warnings.push(Warning {
            code: "RBF_SIGNALING".to_string(),
        });
    }

    // --- Additional safety warnings ---

    // LOW_FEE_RATE: target fee rate < 1 sat/vB — may not confirm
    if target_fee_rate < LOW_FEE_RATE_THRESHOLD {
        warnings.push(Warning {
            code: "LOW_FEE_RATE".to_string(),
        });
    }

    // MANY_INPUTS: more than 50 inputs — expensive and complex
    if num_inputs > MANY_INPUTS_THRESHOLD {
        warnings.push(Warning {
            code: "MANY_INPUTS".to_string(),
        });
    }

    // LARGE_TRANSACTION: > 10,000 vbytes — may face relay limits
    if vbytes > LARGE_TX_VBYTES_THRESHOLD {
        warnings.push(Warning {
            code: "LARGE_TRANSACTION".to_string(),
        });
    }

    // UNECONOMICAL_INPUTS: some selected inputs have negative effective value
    if has_uneconomical_inputs {
        warnings.push(Warning {
            code: "UNECONOMICAL_INPUTS".to_string(),
        });
    }

    warnings
}
