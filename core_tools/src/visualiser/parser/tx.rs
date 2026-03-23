use crate::visualiser::crypto::hash::sha256d;
use crate::visualiser::error::AppError;
use crate::visualiser::parser::varint::read_compact_size;

/// Raw parsed transaction input
#[derive(Debug, Clone)]
pub struct TxInput {
    /// txid in display format (hex, big-endian / reversed)
    pub txid: String,
    /// txid in internal byte order (little-endian, 32 bytes)
    pub txid_bytes: [u8; 32],
    pub vout: u32,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
    /// witness stack items (empty vec for legacy)
    pub witness: Vec<Vec<u8>>,
}

/// Raw parsed transaction output
#[derive(Debug, Clone)]
pub struct TxOutput {
    pub value_sats: u64,
    pub script_pubkey: Vec<u8>,
}

/// Fully parsed raw transaction
#[derive(Debug, Clone)]
pub struct RawTx {
    pub version: u32,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub locktime: u32,
    pub is_segwit: bool,
    /// txid (SHA256d of non-witness serialization), display hex (reversed)
    pub txid: String,
    /// txid bytes in internal order
    pub txid_bytes: [u8; 32],
    /// wtxid (SHA256d of full serialization), None for legacy
    pub wtxid: Option<String>,
    /// total raw byte count (BIP144 full serialization)
    pub size_bytes: usize,
    /// base size: tx without witness data, marker, flag
    pub base_size: usize,
    /// BIP141 weight = base_size * 3 + total_size
    pub weight: u64,
    /// vbytes = ceil(weight / 4)
    pub vbytes: u64,
}

/// Parse a raw Bitcoin transaction from bytes.
/// Returns the RawTx struct.
pub fn parse_tx(data: &[u8]) -> Result<RawTx, AppError> {
    parse_tx_at(data, 0).map(|(tx, _)| tx)
}

/// Parse a raw Bitcoin transaction starting at `offset`.
/// Returns (RawTx, bytes_consumed).
pub fn parse_tx_at(data: &[u8], start: usize) -> Result<(RawTx, usize), AppError> {
    let mut pos = start;

    // Version (4 bytes LE)
    if pos + 4 > data.len() {
        return Err(AppError::invalid_tx(
            "Truncated transaction: missing version",
        ));
    }
    let version = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    pos += 4;

    // Check for SegWit marker + flag
    let is_segwit = pos + 2 <= data.len() && data[pos] == 0x00 && data[pos + 1] == 0x01;
    if is_segwit {
        pos += 2; // consume marker (0x00) and flag (0x01)
    }

    // Input count
    let (in_count, cs) = read_compact_size(data, pos)?;
    pos += cs;
    if in_count == 0 {
        return Err(AppError::invalid_tx("Transaction has zero inputs"));
    }

    let mut inputs = Vec::with_capacity(in_count as usize);
    for _ in 0..in_count {
        // txid (32 bytes, stored LE; display as reversed hex)
        if pos + 32 > data.len() {
            return Err(AppError::invalid_tx("Truncated input: missing txid"));
        }
        let mut txid_bytes = [0u8; 32];
        txid_bytes.copy_from_slice(&data[pos..pos + 32]);
        pos += 32;

        // txid display = reversed
        let mut display = txid_bytes;
        display.reverse();
        let txid_hex = hex::encode(display);

        // vout (4 bytes LE)
        if pos + 4 > data.len() {
            return Err(AppError::invalid_tx("Truncated input: missing vout"));
        }
        let vout = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        // scriptSig
        let (script_len, cs) = read_compact_size(data, pos)?;
        pos += cs;
        if pos + script_len as usize > data.len() {
            return Err(AppError::invalid_tx("Truncated input: missing scriptSig"));
        }
        let script_sig = data[pos..pos + script_len as usize].to_vec();
        pos += script_len as usize;

        // sequence (4 bytes LE)
        if pos + 4 > data.len() {
            return Err(AppError::invalid_tx("Truncated input: missing sequence"));
        }
        let sequence = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        inputs.push(TxInput {
            txid: txid_hex,
            txid_bytes,
            vout,
            script_sig,
            sequence,
            witness: Vec::new(), // filled later if segwit
        });
    }

    // Output count
    let (out_count, cs) = read_compact_size(data, pos)?;
    pos += cs;

    let mut outputs = Vec::with_capacity(out_count as usize);
    for _ in 0..out_count {
        // value (8 bytes LE)
        if pos + 8 > data.len() {
            return Err(AppError::invalid_tx("Truncated output: missing value"));
        }
        let value_sats = u64::from_le_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        // scriptPubKey
        let (script_len, cs) = read_compact_size(data, pos)?;
        pos += cs;
        if pos + script_len as usize > data.len() {
            return Err(AppError::invalid_tx(
                "Truncated output: missing scriptPubKey",
            ));
        }
        let script_pubkey = data[pos..pos + script_len as usize].to_vec();
        pos += script_len as usize;

        outputs.push(TxOutput {
            value_sats,
            script_pubkey,
        });
    }

    // Witness data (one stack per input, only for segwit)
    if is_segwit {
        for inp in inputs.iter_mut() {
            let (stack_items, cs) = read_compact_size(data, pos)?;
            pos += cs;
            let mut witness = Vec::with_capacity(stack_items as usize);
            for _ in 0..stack_items {
                let (item_len, cs) = read_compact_size(data, pos)?;
                pos += cs;
                if pos + item_len as usize > data.len() {
                    return Err(AppError::invalid_tx("Truncated witness item"));
                }
                let item = data[pos..pos + item_len as usize].to_vec();
                pos += item_len as usize;
                witness.push(item);
            }
            inp.witness = witness;
        }
    }

    // Locktime (4 bytes LE)
    if pos + 4 > data.len() {
        return Err(AppError::invalid_tx(
            "Truncated transaction: missing locktime",
        ));
    }
    let locktime = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    pos += 4;

    let total_size = pos - start;

    // Compute base_size: serialize WITHOUT witness data, marker, flag
    let base_size = compute_base_size(&inputs, &outputs);

    // BIP141 weight = base_size * 3 + total_size
    let weight = (base_size as u64) * 3 + (total_size as u64);

    // vbytes = ceil(weight / 4)
    let vbytes = (weight + 3) / 4;

    // Compute txid: SHA256d of non-witness serialization (reversed)
    let non_witness_bytes = serialize_non_witness(version, &inputs, &outputs, locktime);
    let txid_hash = sha256d(&non_witness_bytes);
    let mut txid_display = txid_hash;
    txid_display.reverse();
    let txid = hex::encode(txid_display);

    // Compute wtxid: SHA256d of full serialization (only for segwit)
    let (wtxid, txid_bytes) = if is_segwit {
        let full_bytes = &data[start..pos];
        let wtxid_hash = sha256d(full_bytes);
        let mut wtxid_display = wtxid_hash;
        wtxid_display.reverse();

        let tb_internal = txid_hash; // sha256d output is already LE internal order
        (Some(hex::encode(wtxid_display)), tb_internal)
    } else {
        (None, txid_hash)
    };

    Ok((
        RawTx {
            version,
            inputs,
            outputs,
            locktime,
            is_segwit,
            txid,
            txid_bytes,
            wtxid,
            size_bytes: total_size,
            base_size,
            weight,
            vbytes,
        },
        total_size,
    ))
}

/// Compute base_size: length of legacy (non-witness) serialization
fn compute_base_size(inputs: &[TxInput], outputs: &[TxOutput]) -> usize {
    let mut size = 0;
    size += 4; // version
    size += compact_size_len(inputs.len() as u64);
    for inp in inputs {
        size += 32 + 4; // txid + vout
        size += compact_size_len(inp.script_sig.len() as u64);
        size += inp.script_sig.len();
        size += 4; // sequence
    }
    size += compact_size_len(outputs.len() as u64);
    for out in outputs {
        size += 8; // value
        size += compact_size_len(out.script_pubkey.len() as u64);
        size += out.script_pubkey.len();
    }
    size += 4; // locktime
    size
}

/// Serialize transaction WITHOUT witness data (for txid computation and base_size)
fn serialize_non_witness(
    version: u32,
    inputs: &[TxInput],
    outputs: &[TxOutput],
    locktime: u32,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&version.to_le_bytes());

    // inputs
    write_compact_size(&mut buf, inputs.len() as u64);
    for inp in inputs {
        buf.extend_from_slice(&inp.txid_bytes);
        buf.extend_from_slice(&inp.vout.to_le_bytes());
        write_compact_size(&mut buf, inp.script_sig.len() as u64);
        buf.extend_from_slice(&inp.script_sig);
        buf.extend_from_slice(&inp.sequence.to_le_bytes());
    }

    // outputs
    write_compact_size(&mut buf, outputs.len() as u64);
    for out in outputs {
        buf.extend_from_slice(&out.value_sats.to_le_bytes());
        write_compact_size(&mut buf, out.script_pubkey.len() as u64);
        buf.extend_from_slice(&out.script_pubkey);
    }

    buf.extend_from_slice(&locktime.to_le_bytes());
    buf
}

/// Number of bytes needed to encode `n` as CompactSize
pub fn compact_size_len(n: u64) -> usize {
    if n <= 0xfc {
        1
    } else if n <= 0xffff {
        3
    } else if n <= 0xffff_ffff {
        5
    } else {
        9
    }
}

/// Write a CompactSize integer to a buffer
pub fn write_compact_size(buf: &mut Vec<u8>, n: u64) {
    if n <= 0xfc {
        buf.push(n as u8);
    } else if n <= 0xffff {
        buf.push(0xfd);
        buf.extend_from_slice(&(n as u16).to_le_bytes());
    } else if n <= 0xffff_ffff {
        buf.push(0xfe);
        buf.extend_from_slice(&(n as u32).to_le_bytes());
    } else {
        buf.push(0xff);
        buf.extend_from_slice(&n.to_le_bytes());
    }
}
