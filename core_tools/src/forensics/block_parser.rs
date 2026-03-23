use crate::forensics::crypto::sha256d;
use crate::forensics::error::AppError;
use crate::forensics::tx_parser::{parse_tx_at, read_compact_size, read_varint, RawTx};

pub const MAINNET_MAGIC: u32 = 0xD9B4_BEF9;

/// Parsed 80-byte block header
#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_block_hash: String,
    pub merkle_root: String,
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
    /// block hash in display format (hex, reversed)
    pub block_hash: String,
}

/// A single prevout from undo data
#[derive(Debug, Clone)]
pub struct UndoPrevout {
    pub value_sats: u64,
    pub script_pubkey: Vec<u8>,
}

/// Parsed block with transactions and undo data
#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub header: BlockHeader,
    pub transactions: Vec<RawTx>,
    /// undo prevouts indexed by [tx_idx-1][input_idx] (tx 0 = coinbase, no undo)
    pub undo_prevouts: Vec<Vec<UndoPrevout>>,
    /// BIP34 block height extracted from coinbase
    pub height: u64,
}

#[derive(Debug, Clone)]
pub struct BlockUndoRecord {
    pub prevouts: Vec<Vec<UndoPrevout>>,
    pub checksum: [u8; 32],
    pub payload: Vec<u8>,
}

/// XOR-decode a buffer using the 8-byte key
pub fn xor_decode(data: &mut [u8], xor_key: &[u8; 8], file_offset: usize) {
    if xor_key == &[0u8; 8] {
        return;
    }
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= xor_key[(file_offset + i) % 8];
    }
}

/// Parse all blocks from a blk*.dat file
pub fn parse_blk_file(
    raw_data: &[u8],
    xor_key: &[u8; 8],
) -> Result<Vec<(BlockHeader, Vec<RawTx>)>, AppError> {
    let mut data = raw_data.to_vec();
    xor_decode(&mut data, xor_key, 0);

    let mut pos = 0;
    let mut blocks = Vec::new();

    while pos < data.len() {
        if pos + 4 > data.len() {
            break;
        }

        // Skip zero padding
        if data[pos..pos + 4] == [0, 0, 0, 0] {
            break;
        }

        let magic = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        if magic != MAINNET_MAGIC {
            return Err(AppError::invalid_block(format!(
                "Invalid magic bytes at offset {}: {:08x}",
                pos, magic
            )));
        }
        pos += 4;

        if pos + 4 > data.len() {
            return Err(AppError::invalid_block(
                "Truncated block: missing size field",
            ));
        }
        let block_size =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if pos + block_size > data.len() {
            return Err(AppError::invalid_block(format!(
                "Block data truncated: expected {} bytes, only {} available",
                block_size,
                data.len() - pos
            )));
        }

        let block_data = &data[pos..pos + block_size];

        // Parse header (first 80 bytes)
        let header = parse_block_header(block_data)?;

        // Parse transactions
        let mut bpos = 80;
        let (tx_count, cs) = read_compact_size(block_data, bpos)?;
        bpos += cs;

        let mut transactions = Vec::with_capacity(tx_count as usize);
        for _ in 0..tx_count {
            let (tx, consumed) = parse_tx_at(block_data, bpos)?;
            bpos += consumed;
            transactions.push(tx);
        }

        // Validate coinbase placement
        validate_coinbase_position(&transactions)?;

        // Validate merkle root
        validate_merkle_root(&header, &transactions)?;

        blocks.push((header, transactions));
        pos += block_size;
    }

    Ok(blocks)
}

/// Parse an 80-byte block header
pub fn parse_block_header(data: &[u8]) -> Result<BlockHeader, AppError> {
    if data.len() < 80 {
        return Err(AppError::invalid_block("Block header too short"));
    }

    let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    let mut prev_bytes = [0u8; 32];
    prev_bytes.copy_from_slice(&data[4..36]);
    prev_bytes.reverse();
    let prev_block_hash = hex::encode(prev_bytes);

    let mut merkle_bytes = [0u8; 32];
    merkle_bytes.copy_from_slice(&data[36..68]);
    merkle_bytes.reverse();
    let merkle_root = hex::encode(merkle_bytes);

    let timestamp = u32::from_le_bytes([data[68], data[69], data[70], data[71]]);
    let bits = u32::from_le_bytes([data[72], data[73], data[74], data[75]]);
    let nonce = u32::from_le_bytes([data[76], data[77], data[78], data[79]]);

    let hash = sha256d(&data[..80]);
    let mut hash_display = hash;
    hash_display.reverse();
    let block_hash = hex::encode(hash_display);

    Ok(BlockHeader {
        version,
        prev_block_hash,
        merkle_root,
        timestamp,
        bits,
        nonce,
        block_hash,
    })
}

/// Parse a rev*.dat undo file.
/// Returns undo data for each block in order.
pub fn parse_rev_file(
    raw_data: &[u8],
    xor_key: &[u8; 8],
) -> Result<Vec<BlockUndoRecord>, AppError> {
    let mut data = raw_data.to_vec();
    xor_decode(&mut data, xor_key, 0);

    let mut pos = 0;
    let mut all_blocks = Vec::new();

    while pos < data.len() {
        if pos + 4 > data.len() {
            break;
        }
        if data[pos..pos + 4] == [0, 0, 0, 0] {
            break;
        }

        let magic = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        if magic != MAINNET_MAGIC {
            return Err(AppError::invalid_undo(format!(
                "Invalid magic at offset {}: {:08x}",
                pos, magic
            )));
        }
        pos += 4;

        if pos + 4 > data.len() {
            return Err(AppError::invalid_undo("Truncated undo file: missing size"));
        }
        let record_size =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if pos + record_size + 32 > data.len() {
            return Err(AppError::invalid_undo(format!(
                "Undo record truncated at offset {}",
                pos
            )));
        }

        let payload = &data[pos..pos + record_size];
        let mut checksum = [0u8; 32];
        checksum.copy_from_slice(&data[pos + record_size..pos + record_size + 32]);

        let block_undo = parse_block_undo(payload)?;
        all_blocks.push(BlockUndoRecord {
            prevouts: block_undo,
            checksum,
            payload: payload.to_vec(),
        });

        pos += record_size + 32; // payload + 32-byte checksum
    }

    Ok(all_blocks)
}

/// Parse a CBlockUndo payload
fn parse_block_undo(data: &[u8]) -> Result<Vec<Vec<UndoPrevout>>, AppError> {
    let mut pos = 0;

    let (tx_count, cs) = read_compact_size(data, pos)?;
    pos += cs;

    let mut block_undo = Vec::with_capacity(tx_count as usize);

    for _ in 0..tx_count {
        let (in_count, cs) = read_compact_size(data, pos)?;
        pos += cs;

        let mut tx_undo = Vec::with_capacity(in_count as usize);

        for _ in 0..in_count {
            // VARINT(nHeightAndCoinbase) = nHeight * 2 | fCoinBase
            let (height_and_cb, consumed) = read_varint(data, pos)?;
            pos += consumed;
            let n_height = height_and_cb >> 1;

            // If nHeight > 0, dummy VarInt for backward compat
            if n_height > 0 {
                let (_, consumed) = read_varint(data, pos)?;
                pos += consumed;
            }

            // VARINT(compressed amount)
            let (compressed_amount, consumed) = read_varint(data, pos)?;
            pos += consumed;
            let value_sats = decompress_amount(compressed_amount);

            // Compressed script
            let (script_pubkey, consumed) = decompress_script(data, pos)?;
            pos += consumed;

            tx_undo.push(UndoPrevout {
                value_sats,
                script_pubkey,
            });
        }

        block_undo.push(tx_undo);
    }

    Ok(block_undo)
}

/// Decompress a Bitcoin amount from undo data
pub fn decompress_amount(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut n = n - 1;
    let e = (n % 10) as u32;
    n /= 10;

    let mut x: u64;
    if e < 9 {
        let d = (n % 9) + 1;
        n /= 9;
        x = n * 10 + d;
    } else {
        x = n + 1;
    }

    for _ in 0..e {
        x *= 10;
    }
    x
}

/// Read and decompress a script from undo data
fn decompress_script(data: &[u8], pos: usize) -> Result<(Vec<u8>, usize), AppError> {
    let (n_size, cs) = read_varint(data, pos)?;

    match n_size {
        0 => {
            // P2PKH: 20-byte hash160
            if pos + cs + 20 > data.len() {
                return Err(AppError::invalid_undo("Truncated P2PKH script in undo"));
            }
            let hash = &data[pos + cs..pos + cs + 20];
            let mut script = Vec::with_capacity(25);
            script.extend_from_slice(&[0x76, 0xa9, 0x14]);
            script.extend_from_slice(hash);
            script.extend_from_slice(&[0x88, 0xac]);
            Ok((script, cs + 20))
        }
        1 => {
            // P2SH: 20-byte hash
            if pos + cs + 20 > data.len() {
                return Err(AppError::invalid_undo("Truncated P2SH script in undo"));
            }
            let hash = &data[pos + cs..pos + cs + 20];
            let mut script = Vec::with_capacity(23);
            script.extend_from_slice(&[0xa9, 0x14]);
            script.extend_from_slice(hash);
            script.push(0x87);
            Ok((script, cs + 20))
        }
        2 | 3 => {
            // P2PK compressed: 32-byte x-coordinate
            if pos + cs + 32 > data.len() {
                return Err(AppError::invalid_undo("Truncated P2PK compressed in undo"));
            }
            let x_bytes = &data[pos + cs..pos + cs + 32];
            let prefix = if n_size == 2 { 0x02u8 } else { 0x03u8 };
            let mut script = Vec::with_capacity(35);
            script.push(0x21); // push 33 bytes
            script.push(prefix);
            script.extend_from_slice(x_bytes);
            script.push(0xac); // OP_CHECKSIG
            Ok((script, cs + 32))
        }
        4 | 5 => {
            // P2PK uncompressed: stored as 32-byte x-coordinate
            if pos + cs + 32 > data.len() {
                return Err(AppError::invalid_undo(
                    "Truncated P2PK uncompressed in undo",
                ));
            }
            let x_bytes = &data[pos + cs..pos + cs + 32];
            let compressed_prefix = if n_size == 4 { 0x02u8 } else { 0x03u8 };
            let mut compressed = Vec::with_capacity(33);
            compressed.push(compressed_prefix);
            compressed.extend_from_slice(x_bytes);

            match decompress_pubkey(&compressed) {
                Ok(uncompressed) => {
                    let mut script = Vec::with_capacity(67);
                    script.push(0x41); // push 65 bytes
                    script.extend_from_slice(&uncompressed);
                    script.push(0xac); // OP_CHECKSIG
                    Ok((script, cs + 32))
                }
                Err(_) => {
                    // Fallback: use compressed form if decompression fails
                    let mut script = Vec::with_capacity(35);
                    script.push(0x21);
                    script.extend_from_slice(&compressed);
                    script.push(0xac);
                    Ok((script, cs + 32))
                }
            }
        }
        n => {
            // Raw script: length = n_size - 6
            let script_len = (n - 6) as usize;
            if pos + cs + script_len > data.len() {
                return Err(AppError::invalid_undo(format!(
                    "Truncated raw script in undo: need {} bytes",
                    script_len
                )));
            }
            let script = data[pos + cs..pos + cs + script_len].to_vec();
            Ok((script, cs + script_len))
        }
    }
}

/// Decompress a 33-byte compressed public key to 65-byte uncompressed form
fn decompress_pubkey(compressed: &[u8]) -> Result<Vec<u8>, AppError> {
    use secp256k1::PublicKey;
    let pk = PublicKey::from_slice(compressed)
        .map_err(|e| AppError::invalid_undo(format!("Invalid compressed pubkey: {}", e)))?;
    Ok(pk.serialize_uncompressed().to_vec())
}

/// Validate that the first transaction is a coinbase and no other transaction is a coinbase.
pub fn validate_coinbase_position(transactions: &[RawTx]) -> Result<(), AppError> {
    if transactions.is_empty() {
        return Err(AppError::invalid_block(
            "Block has no transactions (must have at least a coinbase)",
        ));
    }

    if !transactions[0].is_coinbase() {
        return Err(AppError::invalid_block(
            "First transaction in block is not a coinbase transaction",
        ));
    }

    for (i, tx) in transactions.iter().enumerate().skip(1) {
        if tx.is_coinbase() {
            return Err(AppError::invalid_block(format!(
                "Non-first transaction at index {} is a coinbase transaction",
                i
            )));
        }
    }

    Ok(())
}

/// Compute the merkle root from a list of txid byte arrays (in internal byte order).
/// Uses the standard Bitcoin merkle tree construction: SHA256d of concatenated pairs,
/// duplicating the last element if the count is odd, recursively until one hash remains.
pub fn compute_merkle_root(txids: &[[u8; 32]]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }
    if txids.len() == 1 {
        return txids[0];
    }

    let mut current_level: Vec<[u8; 32]> = txids.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));

        // If odd number of elements, duplicate the last one
        if !current_level.len().is_multiple_of(2) {
            let last = *current_level.last().unwrap();
            current_level.push(last);
        }

        for pair in current_level.chunks(2) {
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&pair[0]);
            combined[32..].copy_from_slice(&pair[1]);
            next_level.push(sha256d(&combined));
        }

        current_level = next_level;
    }

    current_level[0]
}

/// Validate that the computed merkle root from transactions matches the block header's merkle root.
pub fn validate_merkle_root(header: &BlockHeader, transactions: &[RawTx]) -> Result<(), AppError> {
    let txids: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.txid_bytes).collect();
    let computed = compute_merkle_root(&txids);

    // Convert computed merkle root to display format (reversed hex) for comparison
    let mut display = computed;
    display.reverse();
    let computed_hex = hex::encode(display);

    if computed_hex != header.merkle_root {
        return Err(AppError::invalid_block(format!(
            "Merkle root mismatch: computed {} but header has {}",
            computed_hex, header.merkle_root
        )));
    }

    Ok(())
}

/// Extract BIP34 block height from coinbase transaction.
/// Returns an error when coinbase scriptSig cannot be decoded as a BIP34 height push.
pub fn extract_bip34_height(coinbase_tx: &RawTx) -> Result<u64, AppError> {
    if coinbase_tx.inputs.is_empty() {
        return Err(AppError::invalid_block(
            "Cannot extract BIP34 height: coinbase has no inputs",
        ));
    }
    let script_sig = &coinbase_tx.inputs[0].script_sig;
    if script_sig.is_empty() {
        return Err(AppError::invalid_block(
            "Cannot extract BIP34 height: empty coinbase scriptSig",
        ));
    }

    let push_len = script_sig[0] as usize;
    if push_len == 0 || push_len > 8 || script_sig.len() < 1 + push_len {
        return Err(AppError::invalid_block(format!(
            "Invalid BIP34 height push in coinbase scriptSig (push_len={}, script_len={})",
            push_len,
            script_sig.len()
        )));
    }

    let mut height: u64 = 0;
    for i in 0..push_len {
        height |= (script_sig[1 + i] as u64) << (8 * i);
    }
    Ok(height)
}
