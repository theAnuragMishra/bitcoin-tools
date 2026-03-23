use crate::forensics::script::{classify_op_return, ScriptType};
use crate::forensics::types::HeuristicResult;

use super::helpers::confidence_label_from_score;
use super::{TxContext, TxSharedData};

/// OP_RETURN detection with deep protocol identification and payload analysis.
///
/// OP_RETURN outputs are provably unspendable data carriers used by various protocols:
/// - **Omni Layer**: Asset issuance and transfer on Bitcoin (marker: "omni" / 0x6f6d6e69)
/// - **OpenTimestamps**: Timestamping proofs (marker: "OTS" / 0x4f5453)
/// - **Stacks**: Smart contract platform (marker: "id:")
/// - **Runes**: Fungible token protocol (marker: OP_13 / 0x5d after OP_RETURN)
/// - **Counterparty**: Asset protocol (marker: "CNTRPRTY")
/// - **VeriBlock**: Proof-of-proof protocol (marker: 0x00 prefix, >40 bytes)
/// - **Data carrier**: Generic data embedding
///
/// Enhanced features:
/// 1. Payload extraction and hex encoding (capped at 160 hex chars = 80 bytes)
/// 2. Data size reporting
/// 3. Multi-output protocol detection (some txs have multiple OP_RETURNs)
/// 4. Confidence scoring based on protocol identification certainty
///
/// Limitations:
/// - Protocol detection is based on known magic bytes; novel protocols will be classified as "data_carrier"
/// - Ordinals inscriptions use witness data, not OP_RETURN — they are NOT detected here
/// - Some OP_RETURN data may be encrypted or use non-standard push opcodes
pub fn detect_op_return(ctx: &TxContext, shared: &TxSharedData) -> HeuristicResult {
    let op_return_indices: Vec<usize> = shared
        .output_types
        .iter()
        .enumerate()
        .filter(|(_, t)| **t == ScriptType::OpReturn)
        .map(|(i, _)| i)
        .collect();

    if op_return_indices.is_empty() {
        return HeuristicResult::not_detected();
    }

    // Analyze each OP_RETURN output
    let mut protocols: Vec<String> = Vec::new();
    let mut total_data_size: usize = 0;
    let mut payload_hexes: Vec<String> = Vec::new();

    for &idx in &op_return_indices {
        let script = &ctx.tx.outputs[idx].script_pubkey;
        let protocol = classify_op_return(script);
        protocols.push(protocol.to_string());

        // Extract payload: skip OP_RETURN opcode (0x6a) and any push length byte
        let payload = extract_op_return_payload(script);
        total_data_size += payload.len();

        // Hex-encode payload, cap at 80 bytes (160 hex chars)
        let hex = hex::encode(&payload[..payload.len().min(80)]);
        if !hex.is_empty() {
            payload_hexes.push(hex);
        }
    }

    // Primary protocol: from the first OP_RETURN
    let primary_protocol = &protocols[0];

    // Numeric confidence
    let conf_score = if *primary_protocol != "data_carrier" && *primary_protocol != "empty" {
        0.9
    } else if total_data_size >= 20 {
        0.6
    } else if total_data_size > 0 {
        0.5
    } else {
        0.3
    };

    let mut result = HeuristicResult::detected();
    result.protocol = Some(primary_protocol.to_string());
    result.output_count = Some(op_return_indices.len());
    result.confidence = Some(confidence_label_from_score(conf_score).to_string());
    result.confidence_score = Some((conf_score * 100.0).round() / 100.0);
    result.data_size = Some(total_data_size);

    // Payload hex from first OP_RETURN
    if !payload_hexes.is_empty() {
        result.payload_hex = Some(payload_hexes[0].clone());
    }

    // All protocols if multiple OP_RETURNs
    if protocols.len() > 1 {
        result.protocols = Some(protocols.clone());
    }

    // Build detail string
    let mut detail_parts = vec![format!(
        "Found {} OP_RETURN output(s) carrying {} total data bytes",
        op_return_indices.len(),
        total_data_size
    )];
    if protocols.len() == 1 {
        detail_parts.push(format!("Detected protocol: {}", primary_protocol));
    } else {
        let proto_list: Vec<String> = protocols.iter().map(|p| p.to_string()).collect();
        detail_parts.push(format!("Detected protocols: {}", proto_list.join(", ")));
    }
    result.details = Some(detail_parts.join(". "));

    result
}

/// Extract the raw payload from an OP_RETURN script.
/// Handles both direct pushdata and length-prefixed pushdata.
fn extract_op_return_payload(script: &[u8]) -> Vec<u8> {
    if script.len() < 2 || script[0] != 0x6a {
        return vec![];
    }

    let after_opreturn = &script[1..];
    if after_opreturn.is_empty() {
        return vec![];
    }

    // Handle OP_RETURN followed by a push length
    let first_byte = after_opreturn[0];

    if first_byte <= 0x4b {
        // Direct push: first byte is the length
        let push_len = first_byte as usize;
        if after_opreturn.len() > push_len {
            return after_opreturn[1..1 + push_len].to_vec();
        } else if after_opreturn.len() > 1 {
            return after_opreturn[1..].to_vec();
        }
    } else if first_byte == 0x4c {
        // OP_PUSHDATA1: next byte is length
        if after_opreturn.len() >= 2 {
            let push_len = after_opreturn[1] as usize;
            let start = 2;
            let end = (start + push_len).min(after_opreturn.len());
            return after_opreturn[start..end].to_vec();
        }
    } else if first_byte == 0x4d {
        // OP_PUSHDATA2: next 2 bytes are length (little-endian)
        if after_opreturn.len() >= 3 {
            let push_len = after_opreturn[1] as usize | ((after_opreturn[2] as usize) << 8);
            let start = 3;
            let end = (start + push_len).min(after_opreturn.len());
            return after_opreturn[start..end].to_vec();
        }
    } else {
        // Could be an opcode like OP_13 (0x5d) for Runes, etc.
        // Return everything after OP_RETURN as payload
        return after_opreturn.to_vec();
    }

    after_opreturn.to_vec()
}
