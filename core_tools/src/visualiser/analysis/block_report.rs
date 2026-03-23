use serde::Serialize;
use std::collections::HashMap;

use crate::visualiser::analysis::tx_report::{build_tx_report, round2, TxReport, VinEntry, VoutEntry};
use crate::visualiser::crypto::merkle::compute_merkle_root;
use crate::visualiser::error::AppError;
use crate::visualiser::fixture::PrevoutEntry;
use crate::visualiser::parser::block::{ParsedBlock, UndoPrevout};
use crate::visualiser::parser::tx::RawTx;
use crate::visualiser::script::address::script_to_address;


#[derive(Debug, Serialize)]
pub struct BlockHeaderReport {
    pub version: u32,
    pub prev_block_hash: String,
    pub merkle_root: String,
    pub merkle_root_valid: bool,
    pub timestamp: u32,
    pub bits: String,
    pub nonce: u32,
    pub block_hash: String,
}

#[derive(Debug, Serialize)]
pub struct CoinbaseInfo {
    pub bip34_height: u64,
    pub coinbase_script_hex: String,
    pub total_output_sats: u64,
}

#[derive(Debug, Serialize)]
pub struct BlockStats {
    pub total_fees_sats: u64,
    pub total_weight: u64,
    pub avg_fee_rate_sat_vb: f64,
    pub script_type_summary: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
pub struct BlockReport {
    pub ok: bool,
    pub mode: String,
    pub block_header: BlockHeaderReport,
    pub tx_count: usize,
    pub coinbase: CoinbaseInfo,
    pub transactions: Vec<TxReport>,
    pub block_stats: BlockStats,
}

/// Decode BIP34 block height from coinbase scriptSig.
/// The scriptSig starts with a minimal push of the height:
///   OP_PUSHBYTES_N followed by N bytes (little-endian).
fn decode_bip34_height(script: &[u8]) -> u64 {
    if script.is_empty() {
        return 0;
    }
    let push_len = script[0] as usize;
    if push_len == 0 || 1 + push_len > script.len() {
        return 0;
    }
    let bytes = &script[1..1 + push_len];
    let mut height: u64 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        height |= (b as u64) << (i * 8);
    }
    height
}

/// Build a TxReport for the coinbase transaction (no fee, no prevouts from undo).
fn build_coinbase_tx_report(tx: &RawTx) -> TxReport {
    use crate::visualiser::analysis::tx_report::{
        PrevoutInfo, RelativeTimelock, SegwitSavings, VinEntry, VoutEntry, Warning,
    };
    use crate::visualiser::script::asm::disassemble;
    use crate::visualiser::script::classify::{classify_input, classify_output};

    let total_output_sats: u64 = tx.outputs.iter().map(|o| o.value_sats).sum();
    let fee_sats = 0u64;
    let fee_rate_sat_vb = 0.0f64;
    let rbf_signaling = tx.inputs.iter().any(|i| i.sequence <= 0xFFFFFFFD);

    let locktime = tx.locktime;
    let locktime_type = if locktime == 0 {
        "none".to_string()
    } else if locktime < 500_000_000 {
        "block_height".to_string()
    } else {
        "unix_timestamp".to_string()
    };

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

    let mut vin = Vec::new();
    for inp in &tx.inputs {
        let script_sig_hex = hex::encode(&inp.script_sig);
        let script_asm = disassemble(&inp.script_sig);
        let witness_hex: Vec<String> = inp.witness.iter().map(|w| hex::encode(w)).collect();

        vin.push(VinEntry {
            txid: inp.txid.clone(),
            vout: inp.vout,
            sequence: inp.sequence,
            script_sig_hex,
            script_asm,
            witness: witness_hex,
            script_type: "unknown".to_string(),
            address: None,
            prevout: PrevoutInfo {
                value_sats: 0,
                script_pubkey_hex: String::new(),
            },
            relative_timelock: RelativeTimelock {
                enabled: false,
                lock_type: None,
                value: None,
            },
            witness_script_asm: None,
        });
    }

    let mut vout = Vec::new();
    for (n, out) in tx.outputs.iter().enumerate() {
        use crate::visualiser::script::asm::{collect_op_return_data, detect_op_return_protocol};
        use serde_json::Value;

        let out_type = classify_output(&out.script_pubkey);
        let address = script_to_address(&out.script_pubkey);
        let script_pubkey_hex = hex::encode(&out.script_pubkey);
        let script_asm = disassemble(&out.script_pubkey);

        let (op_return_data_hex, op_return_data_utf8, op_return_protocol) =
            if out_type == crate::visualiser::script::classify::OutputScriptType::OpReturn {
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

    let mut warnings = Vec::new();
    if rbf_signaling {
        warnings.push(Warning {
            code: "RBF_SIGNALING".to_string(),
        });
    }

    TxReport {
        ok: true,
        network: "mainnet".to_string(),
        segwit: tx.is_segwit,
        txid: tx.txid.clone(),
        wtxid: tx.wtxid.clone(),
        version: tx.version,
        locktime: tx.locktime,
        size_bytes: tx.size_bytes,
        weight: tx.weight,
        vbytes: tx.vbytes,
        total_input_sats: total_output_sats, // coinbase: inputs = outputs (by definition)
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
    }
}

/// Build a complete BlockReport from a ParsedBlock.
pub fn build_block_report(block: &ParsedBlock) -> Result<BlockReport, AppError> {
    let header = &block.header;
    let transactions = &block.transactions;

    if transactions.is_empty() {
        return Err(AppError::invalid_block("Block has no transactions"));
    }

    // Verify merkle root
    let txids: Vec<[u8; 32]> = transactions
        .iter()
        .map(|tx| {
            let decoded = hex::decode(&tx.txid)
                .map_err(|_| AppError::invalid_block("Invalid txid hex in block"))?;
            if decoded.len() != 32 {
                return Err(AppError::invalid_block("Invalid txid length in block"));
            }
            let mut bytes = [0u8; 32];

            // txid is in display (reversed) form; reverse to get internal
            let mut internal = decoded;
            internal.reverse();
            bytes.copy_from_slice(&internal);
            Ok(bytes)
        })
        .collect::<Result<Vec<[u8; 32]>, AppError>>()?;

    let computed_merkle = compute_merkle_root(&txids);
    let mut computed_display = computed_merkle;
    computed_display.reverse();
    let computed_merkle_hex = hex::encode(computed_display);

    let merkle_root_valid = computed_merkle_hex == header.merkle_root;

    if !merkle_root_valid {
        return Err(AppError::merkle_mismatch(
            &header.merkle_root,
            &computed_merkle_hex,
        ));
    }

    // Coinbase = first transaction.
    // Validate: must have exactly one input with txid = 0x00..00 and vout = 0xFFFFFFFF.
    let coinbase_tx = &transactions[0];
    if coinbase_tx.inputs.len() != 1 {
        return Err(AppError::invalid_block(format!(
            "Coinbase transaction must have exactly one input, found {}",
            coinbase_tx.inputs.len()
        )));
    }
    {
        let cb_in = &coinbase_tx.inputs[0];
        let all_zeros = "0".repeat(64);
        if cb_in.txid != all_zeros {
            return Err(AppError::invalid_block(format!(
                "Coinbase input txid must be all zeros, got {}",
                cb_in.txid
            )));
        }
        if cb_in.vout != 0xFFFFFFFF {
            return Err(AppError::invalid_block(format!(
                "Coinbase input vout must be 0xFFFFFFFF, got {}",
                cb_in.vout
            )));
        }
    }
    // Coinbase scriptSig is in inputs[0].script_sig
    let coinbase_script = coinbase_tx.inputs[0].script_sig.clone();
    let bip34_height = decode_bip34_height(&coinbase_script);
    let total_coinbase_output: u64 = coinbase_tx.outputs.iter().map(|o| o.value_sats).sum();

    let coinbase_info = CoinbaseInfo {
        bip34_height,
        coinbase_script_hex: hex::encode(&coinbase_script),
        total_output_sats: total_coinbase_output,
    };

    // Build the block header report
    let block_header_report = BlockHeaderReport {
        version: header.version,
        prev_block_hash: header.prev_block_hash.clone(),
        merkle_root: header.merkle_root.clone(),
        merkle_root_valid,
        timestamp: header.timestamp,
        bits: format!("{:08x}", header.bits),
        nonce: header.nonce,
        block_hash: header.block_hash.clone(),
    };

    // Build tx reports
    // transactions[0] = coinbase (no undo data)
    // transactions[1..] = non-coinbase, paired with undo_prevouts[0], [1], ...
    let mut tx_reports: Vec<TxReport> = Vec::with_capacity(transactions.len());

    // Coinbase report
    let coinbase_report = build_coinbase_tx_report(coinbase_tx);
    tx_reports.push(coinbase_report);

    // Non-coinbase transactions
    for (i, tx) in transactions[1..].iter().enumerate() {
        let tx_undo = block
            .undo_prevouts
            .get(i)
            .ok_or_else(|| AppError::invalid_undo("Missing undo data for transaction"))?;

        // Verify undo entry has the right number of prevouts for this tx's inputs
        if tx_undo.len() != tx.inputs.len() {
            return Err(AppError::invalid_undo(format!(
                "Undo entry for tx {} has {} prevouts but tx has {} inputs",
                i + 1,
                tx_undo.len(),
                tx.inputs.len()
            )));
        }
        // Build prevout map from undo data
        let prevout_entries: Vec<PrevoutEntry> = tx
            .inputs
            .iter()
            .zip(tx_undo.iter())
            .map(|(inp, undo_prev)| PrevoutEntry {
                txid: inp.txid.clone(),
                vout: inp.vout,
                value_sats: undo_prev.value_sats,
                script_pubkey_hex: hex::encode(&undo_prev.script_pubkey),
            })
            .collect();

        let prevout_map: HashMap<(String, u32), &PrevoutEntry> = prevout_entries
            .iter()
            .map(|p| ((p.txid.to_lowercase(), p.vout), p))
            .collect();

        let report = build_tx_report("mainnet", tx, &prevout_map)?;

        tx_reports.push(report);
    }

    // Block stats
    let total_fees_sats: u64 = tx_reports[1..].iter().map(|r| r.fee_sats).sum();
    let total_weight: u64 = tx_reports.iter().map(|r| r.weight).sum();

    // avg fee rate = total_fees / total_vbytes_non_coinbase
    let total_vbytes_non_coinbase: u64 = tx_reports[1..].iter().map(|r| r.vbytes).sum();
    let avg_fee_rate_sat_vb = if total_vbytes_non_coinbase > 0 {
        round2(total_fees_sats as f64 / total_vbytes_non_coinbase as f64)
    } else {
        0.0
    };

    // Script type summary: count all output script types across all transactions
    let mut script_type_summary: HashMap<String, u64> = HashMap::new();
    for tx_report in &tx_reports {
        for vout in &tx_report.vout {
            *script_type_summary
                .entry(vout.script_type.clone())
                .or_insert(0) += 1;
        }
    }

    let block_stats = BlockStats {
        total_fees_sats,
        total_weight,
        avg_fee_rate_sat_vb,
        script_type_summary,
    };

    Ok(BlockReport {
        ok: true,
        mode: "block".to_string(),
        block_header: block_header_report,
        tx_count: transactions.len(),
        coinbase: coinbase_info,
        transactions: tx_reports,
        block_stats,
    })
}

/// Extract the BIP34 block height from a list of transactions (first tx = coinbase).
/// Returns 0 if unable to decode.
fn extract_bip34_height(transactions: &[crate::visualiser::parser::tx::RawTx]) -> u64 {
    let coinbase_script = transactions
        .first()
        .and_then(|tx| tx.inputs.first())
        .map(|inp| &inp.script_sig)
        .filter(|s| !s.is_empty());
    match coinbase_script {
        Some(s) => decode_bip34_height(s),
        None => 0,
    }
}

/// Parse all blocks from blk/rev/xor files and return a list of BlockReports.
///
/// The rev*.dat file stores undo records in BLOCK HEIGHT order, while the
/// blk*.dat file stores blocks in the order they were written to disk (which
/// may differ from height order due to out-of-order block downloads during IBD).
/// We sort the parsed blk blocks by their BIP34 coinbase height before pairing
/// them with rev undo records.
pub fn analyze_block_files(
    blk_data: &[u8],
    rev_data: &[u8],
    xor_key: &[u8; 8],
) -> Result<Vec<BlockReport>, AppError> {
    use crate::visualiser::parser::block::{parse_blk_file, parse_rev_file};

    // Parse the blk file (XOR decode inside)
    let blk_blocks_raw = parse_blk_file(blk_data, xor_key)?;

    // Parse the rev file (XOR decode inside)
    let rev_blocks = parse_rev_file(rev_data, xor_key)?;

    if blk_blocks_raw.len() != rev_blocks.len() {
        return Err(AppError::invalid_undo(format!(
            "blk file has {} blocks but rev file has {} undo records; file is truncated or malformed",
            blk_blocks_raw.len(),
            rev_blocks.len()
        )));
    }

    // Sort blk blocks by BIP34 height so they align with rev records (height order)
    let mut blk_blocks: Vec<_> = blk_blocks_raw
        .into_iter()
        .map(|(header, transactions, raw)| {
            let height = extract_bip34_height(&transactions);
            (height, header, transactions, raw)
        })
        .collect();
    blk_blocks.sort_by_key(|(height, _, _, _)| *height);

    let mut reports = Vec::new();
    //for (block_idx, (_height, header, transactions, _raw)) in blk_blocks.into_iter().enumerate() {
    if let Some((_height, header, transactions, _raw)) = blk_blocks.into_iter().next() {
        let undo_prevouts = rev_blocks
            .get(0)
            .cloned()
            .ok_or_else(|| AppError::invalid_undo("Missing undo data for block"))?;

        let parsed_block = ParsedBlock {
            header,
            transactions,
            undo_prevouts,
        };

        let report = build_block_report(&parsed_block)?;
        reports.push(report);
    }

    Ok(reports)
}
