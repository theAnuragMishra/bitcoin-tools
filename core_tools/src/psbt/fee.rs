use crate::psbt::types::{BuilderError, ScriptType};
use crate::psbt::vbytes;

/// Dust threshold in satoshis
pub const DUST_THRESHOLD: u64 = 546;

/// Result of fee/change computation
#[derive(Debug, Clone)]
pub struct FeeResult {
    /// The fee in satoshis
    pub fee_sats: u64,
    /// The change amount (0 if no change)
    pub change_sats: u64,
    /// Whether a change output should be created
    pub has_change: bool,
    /// Estimated vbytes of the transaction
    pub vbytes: u64,
}

/// Compute fee and change for the transaction.
///
/// This handles the tricky case where adding/removing a change output
/// changes the transaction size and thus the required fee.
///
/// Algorithm:
/// 1. Compute fee WITH change output
/// 2. If change would be >= dust threshold, include it
/// 3. If change < dust threshold, try WITHOUT change
/// 4. Without change, all leftover becomes fee
pub fn compute_fee_and_change(
    input_types: &[ScriptType],
    output_types: &[ScriptType], // payment outputs only
    change_type: &ScriptType,
    total_input: u64,
    total_payments: u64,
    fee_rate: f64,
) -> Result<FeeResult, BuilderError> {
    compute_fee_and_change_with_scripts(
        input_types,
        output_types,
        change_type,
        total_input,
        total_payments,
        fee_rate,
        None,
        None,
    )
}

pub fn compute_fee_and_change_with_scripts(
    input_types: &[ScriptType],
    output_types: &[ScriptType], // payment outputs only
    change_type: &ScriptType,
    total_input: u64,
    total_payments: u64,
    fee_rate: f64,
    output_script_hexes: Option<&[String]>,
    change_script_hex: Option<&str>,
) -> Result<FeeResult, BuilderError> {
    if !fee_rate.is_finite() || fee_rate <= 0.0 {
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            "Fee rate must be a finite positive number",
        ));
    }

    if total_input < total_payments {
        return Err(BuilderError::new(
            "INSUFFICIENT_FUNDS",
            "Total input amount is below payment amount",
        ));
    }

    // Case 1: With change output
    let mut out_types_with_change: Vec<ScriptType> = Vec::with_capacity(output_types.len() + 1);
    out_types_with_change.extend_from_slice(output_types);
    out_types_with_change.push(*change_type);

    let vbytes_with_change = if let (Some(out_scripts), Some(change_script)) =
        (output_script_hexes, change_script_hex)
    {
        let mut outputs: Vec<String> = Vec::with_capacity(out_scripts.len() + 1);
        outputs.extend_from_slice(out_scripts);
        outputs.push(change_script.to_string());
        vbytes::estimate_vbytes_with_outputs(input_types, &outputs)
    } else {
        vbytes::estimate_vbytes(input_types, &out_types_with_change)
    };
    let fee_with_change = required_fee(vbytes_with_change, fee_rate).ok_or_else(|| {
        BuilderError::new(
            "INVALID_FIXTURE",
            "Fee computation overflowed with change output",
        )
    })?;

    let needed_with_change = total_payments.checked_add(fee_with_change).ok_or_else(|| {
        BuilderError::new(
            "INVALID_FIXTURE",
            "Amount arithmetic overflowed while computing needed funds with change",
        )
    })?;

    if total_input >= needed_with_change {
        let change = total_input - needed_with_change;
        if change >= DUST_THRESHOLD {
            // Include change output
            return Ok(FeeResult {
                fee_sats: fee_with_change,
                change_sats: change,
                has_change: true,
                vbytes: vbytes_with_change,
            });
        }
    }

    // Case 2: Without change output — all leftover becomes fee
    let vbytes_no_change = if let Some(out_scripts) = output_script_hexes {
        vbytes::estimate_vbytes_with_outputs(input_types, out_scripts)
    } else {
        vbytes::estimate_vbytes(input_types, output_types)
    };

    // The fee is everything left over after paying outputs
    let min_fee_no_change = required_fee(vbytes_no_change, fee_rate).ok_or_else(|| {
        BuilderError::new(
            "INVALID_FIXTURE",
            "Fee computation overflowed without change output",
        )
    })?;

    let needed_no_change = total_payments
        .checked_add(min_fee_no_change)
        .ok_or_else(|| {
            BuilderError::new(
                "INVALID_FIXTURE",
                "Amount arithmetic overflowed while computing needed funds without change",
            )
        })?;

    if total_input < needed_no_change {
        return Err(BuilderError::new(
            "INSUFFICIENT_FUNDS",
            "Selected inputs cannot satisfy outputs plus minimum required fee",
        ));
    }

    let fee = total_input - total_payments;

    Ok(FeeResult {
        fee_sats: fee,
        change_sats: 0,
        has_change: false,
        vbytes: vbytes_no_change,
    })
}

fn required_fee(vbytes: u64, fee_rate: f64) -> Option<u64> {
    let raw = (vbytes as f64) * fee_rate;
    if !raw.is_finite() || raw < 0.0 || raw > (u64::MAX as f64) {
        return None;
    }
    Some(raw.ceil() as u64)
}
