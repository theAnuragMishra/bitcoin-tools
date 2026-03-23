use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::visualiser::error::AppError;
use crate::visualiser::fixture::{Fixture, PrevoutEntry};
use crate::visualiser::parser::tx::{RawTx, TxInput, TxOutput};
use crate::visualiser::script::address::script_to_address;
use crate::visualiser::script::asm::{collect_op_return_data, detect_op_return_protocol, disassemble};
use crate::visualiser::script::classify::{classify_input, classify_output, InputScriptType, OutputScriptType};

#[derive(Debug, Serialize)]
pub struct RelativeTimelock {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub lock_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct VinEntry {
    pub txid: String,
    pub vout: u32,
    pub sequence: u32,
    pub script_sig_hex: String,
    pub script_asm: String,
    pub witness: Vec<String>,
    pub script_type: String,
    pub address: Option<String>,
    pub prevout: PrevoutInfo,
    pub relative_timelock: RelativeTimelock,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness_script_asm: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PrevoutInfo {
    pub value_sats: u64,
    pub script_pubkey_hex: String,
}

#[derive(Debug, Serialize)]
pub struct VoutEntry {
    pub n: usize,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub script_asm: String,
    pub script_type: String,
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op_return_data_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op_return_data_utf8: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op_return_protocol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SegwitSavings {
    pub witness_bytes: usize,
    pub non_witness_bytes: usize,
    pub total_bytes: usize,
    pub weight_actual: u64,
    pub weight_if_legacy: u64,
    pub savings_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct Warning {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct TxReport {
    pub ok: bool,
    pub network: String,
    pub segwit: bool,
    pub txid: String,
    pub wtxid: Option<String>,
    pub version: u32,
    pub locktime: u32,
    pub size_bytes: usize,
    pub weight: u64,
    pub vbytes: u64,
    pub total_input_sats: u64,
    pub total_output_sats: u64,
    pub fee_sats: u64,
    pub fee_rate_sat_vb: f64,
    pub rbf_signaling: bool,
    pub locktime_type: String,
    pub locktime_value: u32,
    pub segwit_savings: Option<SegwitSavings>,
    pub vin: Vec<VinEntry>,
    pub vout: Vec<VoutEntry>,
    pub warnings: Vec<Warning>,
}

/// Build a complete TxReport from a parsed transaction and prevouts map.
pub fn build_tx_report(
    network: &str,
    tx: &RawTx,
    prevouts_map: &HashMap<(String, u32), &PrevoutEntry>,
) -> Result<TxReport, AppError> {
    // Match prevouts to inputs
    let mut matched_prevouts: Vec<&PrevoutEntry> = Vec::new();
    for inp in &tx.inputs {
        let key = (inp.txid.to_lowercase(), inp.vout);
        match prevouts_map.get(&key) {
            Some(p) => matched_prevouts.push(p),
            None => return Err(AppError::missing_prevout(&inp.txid, inp.vout)),
        }
    }

    // Compute totals
    let total_input_sats: u64 = matched_prevouts.iter().map(|p| p.value_sats).sum();
    let total_output_sats: u64 = tx.outputs.iter().map(|o| o.value_sats).sum();

    if total_output_sats > total_input_sats {
        return Err(AppError::invalid_tx(format!(
            "Output sats ({}) exceed input sats ({})",
            total_output_sats, total_input_sats
        )));
    }
    let fee_sats = total_input_sats - total_output_sats;
    let fee_rate_sat_vb = round2(fee_sats as f64 / tx.vbytes as f64);

    // RBF: any input with sequence <= 0xFFFFFFFD
    let rbf_signaling = tx.inputs.iter().any(|i| i.sequence <= 0xFFFFFFFD);

    // Locktime
    let locktime_type = classify_locktime(tx.locktime);

    // SegWit savings
    let segwit_savings = if tx.is_segwit {
        let non_witness_bytes = tx.base_size;
        let witness_bytes = tx.size_bytes - tx.base_size;
        let total_bytes = tx.size_bytes;
        let weight_actual = tx.weight;
        let weight_if_legacy = (total_bytes as u64) * 4;
        let savings_pct = round2((1.0 - (weight_actual as f64 / weight_if_legacy as f64)) * 100.0);
        Some(SegwitSavings {
            witness_bytes,
            non_witness_bytes,
            total_bytes,
            weight_actual,
            weight_if_legacy,
            savings_pct,
        })
    } else {
        None
    };

    // Build vin entries
    let mut vin = Vec::new();
    for (i, inp) in tx.inputs.iter().enumerate() {
        let prevout = matched_prevouts[i];
        let prevout_script = hex::decode(&prevout.script_pubkey_hex)
            .map_err(|_| AppError::invalid_fixture("Invalid prevout script hex"))?;

        let script_type = classify_input(&prevout_script, &inp.script_sig, &inp.witness);
        let address = script_to_address(&prevout_script);

        let script_sig_hex = hex::encode(&inp.script_sig);
        let script_asm = disassemble(&inp.script_sig);

        // Witness items as hex strings (empty item = "")
        let witness_hex: Vec<String> = inp.witness.iter().map(|w| hex::encode(w)).collect();

        // witness_script_asm for P2WSH and P2SH-P2WSH
        let witness_script_asm = match &script_type {
            InputScriptType::P2WSH | InputScriptType::P2SHP2WSH => {
                // Last witness item is the witness script
                inp.witness.last().map(|ws| disassemble(ws))
            }
            _ => None,
        };

        let relative_timelock = compute_relative_timelock(inp.sequence, tx.version);

        vin.push(VinEntry {
            txid: inp.txid.clone(),
            vout: inp.vout,
            sequence: inp.sequence,
            script_sig_hex,
            script_asm,
            witness: witness_hex,
            script_type: script_type.as_str().to_string(),
            address,
            prevout: PrevoutInfo {
                value_sats: prevout.value_sats,
                script_pubkey_hex: prevout.script_pubkey_hex.clone(),
            },
            relative_timelock,
            witness_script_asm,
        });
    }

    // Build vout entries
    let mut vout = Vec::new();
    for (n, out) in tx.outputs.iter().enumerate() {
        let out_type = classify_output(&out.script_pubkey);
        let address = script_to_address(&out.script_pubkey);
        let script_pubkey_hex = hex::encode(&out.script_pubkey);
        let script_asm = disassemble(&out.script_pubkey);

        let (op_return_data_hex, op_return_data_utf8, op_return_protocol) =
            if out_type == OutputScriptType::OpReturn {
                let data = collect_op_return_data(&out.script_pubkey);
                let hex_str = hex::encode(&data);
                let utf8_val: Value = match std::str::from_utf8(&data) {
                    Ok(s) => Value::String(s.to_string()),
                    Err(_) => Value::Null,
                };
                let protocol = detect_op_return_protocol(&data).to_string();
                (Some(hex_str), Some(utf8_val), Some(protocol))
            } else {
                (None, None, None)
            };

        vout.push(VoutEntry {
            n,
            value_sats: out.value_sats,
            script_pubkey_hex,
            script_asm,
            script_type: out_type.as_str().to_string(),
            address,
            op_return_data_hex,
            op_return_data_utf8,
            op_return_protocol,
        });
    }

    // Build warnings
    let mut warnings = Vec::new();
    if rbf_signaling {
        warnings.push(Warning {
            code: "RBF_SIGNALING".to_string(),
        });
    }
    if fee_sats > 1_000_000 || fee_rate_sat_vb > 200.0 {
        warnings.push(Warning {
            code: "HIGH_FEE".to_string(),
        });
    }
    let has_dust = vout
        .iter()
        .any(|o| o.script_type != "op_return" && o.value_sats < 546);
    if has_dust {
        warnings.push(Warning {
            code: "DUST_OUTPUT".to_string(),
        });
    }
    let has_unknown = vout.iter().any(|o| o.script_type == "unknown");
    if has_unknown {
        warnings.push(Warning {
            code: "UNKNOWN_OUTPUT_SCRIPT".to_string(),
        });
    }

    Ok(TxReport {
        ok: true,
        network: network.to_string(),
        segwit: tx.is_segwit,
        txid: tx.txid.clone(),
        wtxid: tx.wtxid.clone(),
        version: tx.version,
        locktime: tx.locktime,
        size_bytes: tx.size_bytes,
        weight: tx.weight,
        vbytes: tx.vbytes,
        total_input_sats,
        total_output_sats,
        fee_sats,
        fee_rate_sat_vb,
        rbf_signaling,
        locktime_type,
        locktime_value: tx.locktime,
        segwit_savings,
        vin,
        vout,
        warnings,
    })
}

/// Run the full analyze pipeline from a fixture JSON string.
pub fn analyze_fixture(fixture_json: &str) -> Result<TxReport, AppError> {
    let fixture = Fixture::from_json(fixture_json)?;
    let raw_bytes = hex::decode(&fixture.raw_tx)
        .map_err(|e| AppError::invalid_tx(format!("Invalid raw_tx hex: {}", e)))?;

    let tx = crate::visualiser::parser::tx::parse_tx(&raw_bytes)?;

    let prevout_map = fixture.prevout_map()?;

    // Verify all prevouts are consumed (no extra prevouts)
    let input_keys: std::collections::HashSet<(String, u32)> = tx
        .inputs
        .iter()
        .map(|i| (i.txid.to_lowercase(), i.vout))
        .collect();

    for p in &fixture.prevouts {
        let key = (p.txid.to_lowercase(), p.vout);
        if !input_keys.contains(&key) {
            return Err(AppError::new(
                crate::visualiser::error::ErrorCode::InvalidFixture,
                format!(
                    "Prevout {}:{} does not correspond to any input",
                    p.txid, p.vout
                ),
            ));
        }
    }

    build_tx_report(&fixture.network, &tx, &prevout_map)
}

/// Classify locktime into type string and value
fn classify_locktime(locktime: u32) -> String {
    if locktime == 0 {
        "none".to_string()
    } else if locktime < 500_000_000 {
        "block_height".to_string()
    } else {
        "unix_timestamp".to_string()
    }
}

/// Compute per-input relative timelock (BIP68)
fn compute_relative_timelock(sequence: u32, tx_version: u32) -> RelativeTimelock {
    // BIP68 only applies for tx version >= 2
    // Bit 31 (0x80000000) set = disable relative timelock
    if tx_version < 2 || (sequence & 0x8000_0000) != 0 {
        return RelativeTimelock {
            enabled: false,
            lock_type: None,
            value: None,
        };
    }

    // Bit 22 (0x00400000): 1 = time, 0 = blocks
    if (sequence & 0x0040_0000) != 0 {
        // Time-based: low 16 bits * 512 seconds
        let seconds = ((sequence & 0x0000_FFFF) as u64) * 512;
        RelativeTimelock {
            enabled: true,
            lock_type: Some("time".to_string()),
            value: Some(seconds),
        }
    } else {
        // Block-based: low 16 bits
        let blocks = (sequence & 0x0000_FFFF) as u64;
        RelativeTimelock {
            enabled: true,
            lock_type: Some("blocks".to_string()),
            value: Some(blocks),
        }
    }
}

/// Round to 2 decimal places
pub fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}