use crate::psbt::types::ScriptType;

/// Estimated input size in virtual bytes (vbytes) for spending a UTXO of a given script type.
///
/// These estimates assume:
/// - P2PKH: 148 vbytes (outpoint 36 + scriptSigLen 1 + scriptSig 107 + nSequence 4)
/// - P2WPKH: 68 vbytes (outpoint 36 + scriptSigLen 1 + nSequence 4 + witness: (1 + 73 + 34)/4 = 27)
/// - P2SH-P2WPKH: 91 vbytes (outpoint 36 + scriptSigLen 1 + scriptSig 23 + nSequence 4 + witness: (1 + 73 + 34)/4 = 27)
/// - P2TR: 58 vbytes (outpoint 36 + scriptSigLen 1 + nSequence 4 + witness: (1 + 65)/4 = 16.5 → 17)
/// - P2WSH: 131 vbytes (conservative upper bound when witnessScript is unknown)
/// - P2SH: 320 vbytes (conservative upper bound when redeemScript is unknown)
///
/// For the fractional witness discount, we compute using weight units then convert.
/// weight = non_witness_bytes * 4 + witness_bytes
/// vbytes = ceil(weight / 4)
///
/// We compute per-input weight contribution (not including tx overhead).
pub fn input_weight(script_type: &ScriptType) -> u64 {
    match script_type {
        // P2PKH: all non-witness
        // Non-witness: outpoint(36) + scriptSigLen(1) + scriptSig(107) + nSequence(4) = 148 bytes
        // Weight = 148 * 4 = 592
        ScriptType::P2pkh => 592,

        // P2WPKH:
        // Non-witness: outpoint(36) + scriptSigLen(1) + nSequence(4) = 41 bytes
        // Witness: items_count(1) + sig_len(1) + sig(72) + pubkey_len(1) + pubkey(33) = 108 bytes
        // Weight = 41 * 4 + 108 = 164 + 108 = 272
        ScriptType::P2wpkh => 272,

        // P2SH-P2WPKH:
        // Non-witness: outpoint(36) + scriptSigLen(1) + scriptSig(23) + nSequence(4) = 64 bytes
        //   scriptSig = push(1) + redeemScript(22) = 23
        // Witness: items_count(1) + sig_len(1) + sig(72) + pubkey_len(1) + pubkey(33) = 108 bytes
        // Weight = 64 * 4 + 108 = 256 + 108 = 364
        ScriptType::P2shP2wpkh => 364,

        // P2SH-P2WSH (conservative estimate):
        // Use a higher estimate to avoid underpaying fees when redeemScript/witnessScript are unknown.
        // Weight = 64 * 4 + 360 = 616
        ScriptType::P2shP2wsh => 616,

        // P2TR (key path spend):
        // Non-witness: outpoint(36) + scriptSigLen(1) + nSequence(4) = 41 bytes
        // Witness: items_count(1) + sig_len(1) + sig(64) = 66 bytes
        // Weight = 41 * 4 + 66 = 164 + 66 = 230
        ScriptType::P2tr => 230,

        // P2WSH (conservative estimate):
        // Use a higher estimate to avoid underpaying fees when witnessScript is unknown.
        // Weight = 41*4 + 360 = 524
        ScriptType::P2wsh => 524,

        // P2SH (conservative estimate):
        // Use a higher estimate to avoid underpaying fees when redeemScript is unknown.
        // Weight = 320 * 4 = 1280
        ScriptType::P2sh => 1280,

        // OP_RETURN is not a spendable input script type.
        ScriptType::OpReturn => 0,
    }
}

/// Estimated output size in weight units for a given script type.
pub fn output_weight(script_type: &ScriptType) -> u64 {
    // Outputs are always non-witness, so weight = size * 4
    let size = match script_type {
        // nValue(8) + scriptLen(1) + script(25) = 34
        ScriptType::P2pkh => 34,
        // nValue(8) + scriptLen(1) + script(22) = 31
        ScriptType::P2wpkh => 31,
        // nValue(8) + scriptLen(1) + script(23) = 32
        ScriptType::P2shP2wpkh | ScriptType::P2shP2wsh | ScriptType::P2sh => 32,
        // nValue(8) + scriptLen(1) + script(34) = 43
        ScriptType::P2tr | ScriptType::P2wsh => 43,
        // OP_RETURN (nulldata). The script length is variable; we use a safe default
        // based on standard 1-byte push up to 80 bytes of data.
        // nValue(8) + scriptLen(1) + script(1 + 1 + 80) = 91 bytes
        ScriptType::OpReturn => 91,
    };
    size * 4
}

/// Estimated output size in weight units for a specific script_pubkey_hex.
pub fn output_weight_for_script(script_hex: &str) -> u64 {
    let script_len = script_hex.len() / 2;
    let size = 8 + varint_size(script_len as u64) + script_len as u64;
    size * 4
}

/// Estimate vbytes for a transaction using exact output script sizes.
pub fn estimate_vbytes_with_outputs(
    input_types: &[ScriptType],
    output_script_hexes: &[String],
) -> u64 {
    let outputs_weight: u64 = output_script_hexes
        .iter()
        .map(|s| output_weight_for_script(s))
        .sum();
    let has_segwit = input_types.iter().any(|t| t.is_segwit());
    let overhead = overhead_weight(input_types.len(), output_script_hexes.len(), has_segwit);
    let inputs_weight: u64 = input_types.iter().map(|t| input_weight(t)).sum();
    weight_to_vbytes(overhead + inputs_weight + outputs_weight)
}

/// Transaction overhead weight.
/// For segwit transactions: nVersion(4) + marker(1) + flag(1) + input_count(varint) + output_count(varint) + nLockTime(4)
/// The marker and flag bytes are witness data, so they get the witness discount.
/// Non-witness overhead: nVersion(4) + input_count(varint) + output_count(varint) + nLockTime(4)
/// Witness overhead: marker(1) + flag(1)
///
/// For non-segwit (all p2pkh inputs): nVersion(4) + input_count(varint) + output_count(varint) + nLockTime(4)
///
/// varint for count up to 252 is 1 byte.
/// For larger counts (253-65535) it's 3 bytes.
pub fn overhead_weight(num_inputs: usize, num_outputs: usize, has_segwit: bool) -> u64 {
    let input_count_size = varint_size(num_inputs as u64);
    let output_count_size = varint_size(num_outputs as u64);

    // Non-witness: nVersion(4) + input_count + output_count + nLockTime(4)
    let non_witness = 4 + input_count_size + output_count_size + 4;

    if has_segwit {
        // Witness: marker(1) + flag(1) = 2 bytes of witness data
        non_witness * 4 + 2
    } else {
        non_witness * 4
    }
}

/// CompactSize/varint encoding size in bytes
pub(crate) fn varint_size(n: u64) -> u64 {
    if n < 253 {
        1
    } else if n <= 0xFFFF {
        3
    } else if n <= 0xFFFF_FFFF {
        5
    } else {
        9
    }
}

/// Estimate total transaction weight given input types and output types.
/// Returns weight units.
pub(crate) fn estimate_tx_weight(input_types: &[ScriptType], output_types: &[ScriptType]) -> u64 {
    let has_segwit = input_types.iter().any(|t| t.is_segwit());

    let overhead = overhead_weight(input_types.len(), output_types.len(), has_segwit);

    let inputs_weight: u64 = input_types.iter().map(|t| input_weight(t)).sum();
    let outputs_weight: u64 = output_types.iter().map(|t| output_weight(t)).sum();

    overhead + inputs_weight + outputs_weight
}

/// Convert weight units to vbytes (ceil)
pub fn weight_to_vbytes(weight: u64) -> u64 {
    (weight + 3) / 4
}

/// Estimate vbytes for a transaction
pub fn estimate_vbytes(input_types: &[ScriptType], output_types: &[ScriptType]) -> u64 {
    let weight = estimate_tx_weight(input_types, output_types);
    weight_to_vbytes(weight)
}
