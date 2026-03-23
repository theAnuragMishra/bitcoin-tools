use std::collections::HashSet;

// use bitcoin::PrivateKey;
use bitcoin::{Address, Network, ScriptBuf};

use crate::psbt::fee::DUST_THRESHOLD;
use crate::psbt::types::{BuilderError, Fixture, ScriptType};

/// Maximum single UTXO or payment value: 21 million BTC in sats
const MAX_SATS: u64 = 21_000_000 * 100_000_000;

/// Derive a Bitcoin address from a script_pubkey_hex, if possible.
/// Returns None for non-standard scripts (e.g., OP_RETURN).
pub fn derive_address(script_hex: &str, network_str: &str) -> Option<String> {
    let script = ScriptBuf::from_hex(script_hex).ok()?;
    let network = match network_str {
        "mainnet" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => return None,
    };
    Address::from_script(&script, network)
        .ok()
        .map(|a| a.to_string())
}

/// Parse and validate a fixture from JSON string
pub fn parse_fixture(json_str: &str) -> Result<Fixture, BuilderError> {
    let mut fixture: Fixture = serde_json::from_str(json_str).map_err(|e| {
        BuilderError::new(
            "INVALID_FIXTURE",
            &format!("Failed to parse fixture JSON: {}", e),
        )
    })?;

    validate_fixture(&mut fixture)?;

    Ok(fixture)
}

/// Validate fixture fields defensively
fn validate_fixture(fixture: &mut Fixture) -> Result<(), BuilderError> {
    // Network must be valid
    match fixture.network.as_str() {
        "mainnet" | "testnet" | "signet" | "regtest" => {}
        _ => {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("Unknown network: {}", fixture.network),
            ));
        }
    }

    // Must have at least one UTXO
    if fixture.utxos.is_empty() {
        return Err(BuilderError::new("INVALID_FIXTURE", "No UTXOs provided"));
    }

    // Must have at least one payment
    if fixture.payments.is_empty() {
        return Err(BuilderError::new("INVALID_FIXTURE", "No payments provided"));
    }

    // Check for overflow in total UTXO amounts
    let mut total_utxos: u64 = 0;

    // Validate UTXOs
    for (i, utxo) in fixture.utxos.iter_mut().enumerate() {
        if utxo.txid.len() != 64 {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("UTXO {} has invalid txid length: {}", i, utxo.txid.len()),
            ));
        }
        // Validate txid is valid hex
        utxo.txid.parse::<bitcoin::Txid>().map_err(|_| {
            BuilderError::new(
                "INVALID_FIXTURE",
                &format!("UTXO {} has invalid txid hex", i),
            )
        })?;
        if utxo.value_sats == 0 {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("UTXO {} has zero value", i),
            ));
        }
        // Reject values above 21M BTC
        if utxo.value_sats > MAX_SATS {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!(
                    "UTXO {} has value exceeding maximum supply: {} sats",
                    i, utxo.value_sats
                ),
            ));
        }
        total_utxos = total_utxos.checked_add(utxo.value_sats).ok_or_else(|| {
            BuilderError::new(
                "INVALID_FIXTURE",
                &format!("Total UTXO amounts overflow at UTXO {}", i),
            )
        })?;
        if utxo.script_pubkey_hex.is_empty() {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("UTXO {} has empty script_pubkey_hex", i),
            ));
        }
        // Validate script_pubkey_hex and derive script_type
        utxo.script_type = classify_script_pubkey(
            &utxo.script_pubkey_hex,
            Some(&utxo.script_type),
            &format!("UTXO {}", i),
        )?;
        if matches!(utxo.script_type, ScriptType::OpReturn) {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("UTXO {} has OP_RETURN script", i),
            ));
        }

        // Derive address from script_pubkey_hex (authoritative source)
        // Silently replace any fixture-provided address with the derived one
        utxo.address = derive_address(&utxo.script_pubkey_hex, &fixture.network);
    }

    // Validate payments
    for (i, payment) in fixture.payments.iter_mut().enumerate() {
        if payment.script_pubkey_hex.is_empty() {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("Payment {} has empty script_pubkey_hex", i),
            ));
        }

        payment.script_type = classify_script_pubkey(
            &payment.script_pubkey_hex,
            Some(&payment.script_type),
            &format!("Payment {}", i),
        )?;

        if matches!(payment.script_type, ScriptType::OpReturn) {
            if payment.value_sats != 0 {
                return Err(BuilderError::new(
                    "INVALID_FIXTURE",
                    &format!("Payment {} OP_RETURN must have value_sats = 0", i),
                ));
            }
        } else {
            if payment.value_sats == 0 {
                return Err(BuilderError::new(
                    "INVALID_FIXTURE",
                    &format!("Payment {} has zero value", i),
                ));
            }
            if payment.value_sats > MAX_SATS {
                return Err(BuilderError::new(
                    "INVALID_FIXTURE",
                    &format!(
                        "Payment {} has value exceeding maximum supply: {} sats",
                        i, payment.value_sats
                    ),
                ));
            }
        }

        // Derive address from script_pubkey_hex (authoritative source)
        payment.address = derive_address(&payment.script_pubkey_hex, &fixture.network);
    }

    // Validate change template
    if fixture.change.script_pubkey_hex.is_empty() {
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            "Change template has empty script_pubkey_hex",
        ));
    }
    fixture.change.script_type = classify_script_pubkey(
        &fixture.change.script_pubkey_hex,
        Some(&fixture.change.script_type),
        "Change template",
    )?;
    if matches!(fixture.change.script_type, ScriptType::OpReturn) {
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            "Change template cannot be OP_RETURN",
        ));
    }
    // Derive address for change template from script_pubkey_hex
    fixture.change.address = derive_address(&fixture.change.script_pubkey_hex, &fixture.network);

    // Fee rate must be finite and positive (NaN comparisons always return false,
    // so `<= 0.0` alone does NOT catch NaN)
    if !fixture.fee_rate_sat_vb.is_finite() || fixture.fee_rate_sat_vb <= 0.0 {
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            "Fee rate must be a finite positive number",
        ));
    }

    // Reject duplicate UTXOs (same txid + vout)
    let mut seen_utxos = HashSet::new();
    for (i, utxo) in fixture.utxos.iter().enumerate() {
        let key = (utxo.txid.as_str(), utxo.vout);
        if !seen_utxos.insert(key) {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("Duplicate UTXO at index {}: {}:{}", i, utxo.txid, utxo.vout),
            ));
        }
    }

    // Reject dust payment amounts (we cannot create outputs below dust threshold)
    for (i, payment) in fixture.payments.iter().enumerate() {
        if matches!(payment.script_type, ScriptType::OpReturn) {
            continue;
        }
        if payment.value_sats < DUST_THRESHOLD {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!(
                    "Payment {} has dust amount: {} sats (minimum {})",
                    i, payment.value_sats, DUST_THRESHOLD
                ),
            ));
        }
    }

    // current_height is a block height, not a timestamp
    if let Some(ch) = fixture.current_height {
        if ch >= 500_000_000 {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                "current_height must be a block height (< 500_000_000)",
            ));
        }
    }

    // Reject max_inputs of 0 (impossible to build a transaction)
    if let Some(ref policy) = fixture.policy {
        if let Some(max) = policy.max_inputs {
            if max == 0 {
                return Err(BuilderError::new(
                    "INVALID_FIXTURE",
                    "Policy max_inputs cannot be 0",
                ));
            }
        }
    }

    // Part of stretch goals

    // if let Some(ref wifs) = fixture.test_signing_wifs {
    //     if wifs.is_empty() {
    //         return Err(BuilderError::new(
    //             "INVALID_FIXTURE",
    //             "test_signing_wifs cannot be an empty array",
    //         ));
    //     }
    //     for (i, wif) in wifs.iter().enumerate() {
    //         PrivateKey::from_wif(wif).map_err(|e| {
    //             BuilderError::new(
    //                 "INVALID_FIXTURE",
    //                 &format!("test_signing_wifs[{}] is not a valid WIF key: {}", i, e),
    //             )
    //         })?;
    //     }
    // }

    // Check for overflow in total payment amounts
    let mut total_payments: u64 = 0;
    for (i, payment) in fixture.payments.iter().enumerate() {
        total_payments = total_payments
            .checked_add(payment.value_sats)
            .ok_or_else(|| {
                BuilderError::new(
                    "INVALID_FIXTURE",
                    &format!("Total payment amounts overflow at payment {}", i),
                )
            })?;
    }

    Ok(())
}

/// Parse a hex script_pubkey string using the bitcoin library, then classify it
/// using bitcoin's own script-type predicates.
///
/// `script_pubkey_hex` is authoritative — the `declared` type is only consulted
/// for the P2SH vs P2SH-P2WPKH vs P2SH-P2WSH ambiguity (all three share the
/// same on-chain script template.

fn classify_script_pubkey(
    script_hex: &str,
    declared: Option<&ScriptType>,
    context: &str,
) -> Result<ScriptType, BuilderError> {
    let script = ScriptBuf::from_hex(script_hex).map_err(|e| {
        BuilderError::new(
            "INVALID_FIXTURE",
            &format!("{} has invalid script_pubkey_hex: {}", context, e),
        )
    })?;

    if script.is_p2pkh() {
        return Ok(ScriptType::P2pkh);
    }

    if script.is_p2wpkh() {
        return Ok(ScriptType::P2wpkh);
    }

    if script.is_p2wsh() {
        return Ok(ScriptType::P2wsh);
    }

    if script.is_p2tr() {
        return Ok(ScriptType::P2tr);
    }

    // P2SH template is shared by plain P2SH, P2SH-P2WPKH, and P2SH-P2WSH.
    // The on-chain scriptPubKey is identical; the declared type disambiguates.
    if script.is_p2sh() {
        match declared {
            Some(ScriptType::P2shP2wpkh) => return Ok(ScriptType::P2shP2wpkh),
            Some(ScriptType::P2shP2wsh) => return Ok(ScriptType::P2shP2wsh),
            _ => return Ok(ScriptType::P2sh),
        }
    }

    // OP_RETURN: bitcoin's is_op_return() only checks the leading opcode.
    // We additionally validate the push-data structure and enforce the 80-byte
    // consensus limit
    if script.is_op_return() {
        validate_op_return_script(script.as_bytes(), context)?;
        return Ok(ScriptType::OpReturn);
    }

    // No known standard template matched.
    Err(BuilderError::new(
        "INVALID_FIXTURE",
        &format!(
            "{} script_pubkey_hex does not match a supported standard script",
            context
        ),
    ))
}

/// Validate the structure and size of an OP_RETURN script payload.

/// - Must have at least one push-data byte after the OP_RETURN opcode.
/// - The push opcode (direct, PUSHDATA1, PUSHDATA2, PUSHDATA4) must be
///   self-consistent with the remaining bytes.
/// - Payload must not exceed 80 bytes.
fn validate_op_return_script(bytes: &[u8], context: &str) -> Result<(), BuilderError> {
    // bytes[0] == OP_RETURN (0x6a) — already confirmed by the caller.
    if bytes.len() < 2 {
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            &format!("{} OP_RETURN missing pushdata", context),
        ));
    }

    let op = bytes[1];
    let (header_len, data_len) = if op <= 75 {
        // Direct push: opcode value == data length
        (2usize, op as usize)
    } else if op == 0x4c {
        // PUSHDATA1: next byte is length
        if bytes.len() < 3 {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("{} OP_RETURN PUSHDATA1 missing length", context),
            ));
        }
        (3usize, bytes[2] as usize)
    } else if op == 0x4d {
        // PUSHDATA2: next two bytes (little-endian) are length
        if bytes.len() < 4 {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("{} OP_RETURN PUSHDATA2 missing length", context),
            ));
        }
        (4usize, u16::from_le_bytes([bytes[2], bytes[3]]) as usize)
    } else if op == 0x4e {
        // PUSHDATA4: next four bytes (little-endian) are length
        if bytes.len() < 6 {
            return Err(BuilderError::new(
                "INVALID_FIXTURE",
                &format!("{} OP_RETURN PUSHDATA4 missing length", context),
            ));
        }
        (
            6usize,
            u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]) as usize,
        )
    } else {
        // Non-push opcode after OP_RETURN — not a standard nulldata script.
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            &format!(
                "{} OP_RETURN has non-standard opcode after the return marker",
                context
            ),
        ));
    };

    if bytes.len() != header_len + data_len {
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            &format!("{} OP_RETURN length mismatch", context),
        ));
    }
    if data_len > 80 {
        return Err(BuilderError::new(
            "INVALID_FIXTURE",
            &format!("{} OP_RETURN data exceeds 80 bytes", context),
        ));
    }

    Ok(())
}
