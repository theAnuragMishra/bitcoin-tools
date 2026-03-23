use crate::visualiser::crypto::hash::sha256d;
use crate::visualiser::error::AppError;
use crate::visualiser::parser::tx::{parse_tx_at, RawTx};
use crate::visualiser::parser::varint::{read_compact_size, read_varint};

pub const MAINNET_MAGIC: u32 = 0xD9B4BEF9; // stored LE in file = F9 BE B4 D9

/// Parsed 80-byte block header
#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub version: u32,
    /// prev block hash in display format (hex, reversed)
    pub prev_block_hash: String,
    /// merkle root in display format (hex, reversed)
    pub merkle_root: String,
    /// merkle root in internal byte order
    pub merkle_root_bytes: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
    /// block hash in display format (hex, reversed)
    pub block_hash: String,
    /// block hash in internal byte order
    pub block_hash_bytes: [u8; 32],
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
    /// undo prevouts for all non-coinbase inputs, indexed by [tx_idx-1][input_idx]
    /// tx_idx 0 = coinbase (no undo), tx_idx 1.. = non-coinbase
    pub undo_prevouts: Vec<Vec<UndoPrevout>>,
}

/// XOR-decode a buffer using the 8-byte key, starting at absolute file offset `file_offset`
pub fn xor_decode(data: &mut [u8], xor_key: &[u8; 8], file_offset: usize) {
    if xor_key == &[0u8; 8] {
        return; // all-zero key = no-op
    }
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= xor_key[(file_offset + i) % 8];
    }
}

/// Parse all blocks from a blk*.dat file (already XOR-decoded)
/// Returns a list of (block_data_slice_start, block_data_slice_end) within `data`
/// Actually returns full parsed header + transactions per block
pub fn parse_blk_file(
    raw_data: &[u8],
    xor_key: &[u8; 8],
) -> Result<Vec<(BlockHeader, Vec<RawTx>, Vec<u8>)>, AppError> {
    // First XOR-decode the entire file
    let mut data = raw_data.to_vec();
    xor_decode(&mut data, xor_key, 0);

    let mut pos = 0;
    let mut blocks = Vec::new();

    while pos < data.len() {
        // Skip any zero padding between blocks
        if pos + 4 > data.len() {
            break;
        }

        // Check for zero padding
        if data[pos..pos + 4] == [0, 0, 0, 0] {
            break;
        }

        // Magic bytes (4 bytes): 0xF9BEB4D9 for mainnet stored as F9 BE B4 D9
        let magic = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        if magic != MAINNET_MAGIC {
            return Err(AppError::invalid_block(format!(
                "Invalid magic bytes at offset {}: {:08x}",
                pos, magic
            )));
        }
        pos += 4;

        // Block size (4 bytes LE)
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

        let block_start = pos;
        let block_data = &data[pos..pos + block_size];

        // Parse the block header (first 80 bytes)
        let header = parse_block_header(block_data)?;

        // Parse transactions (CompactSize tx count + raw txs)
        let mut bpos = 80;
        let (tx_count, cs) = read_compact_size(block_data, bpos)?;
        bpos += cs;

        let mut transactions = Vec::with_capacity(tx_count as usize);
        for _ in 0..tx_count {
            let (tx, consumed) = parse_tx_at(block_data, bpos)?;
            bpos += consumed;
            transactions.push(tx);
        }

        // Save the raw block bytes (for undo pairing later)
        let block_bytes = block_data.to_vec();
        blocks.push((header, transactions, block_bytes));

        pos = block_start + block_size;
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
    let mut prev_display = prev_bytes;
    prev_display.reverse();
    let prev_block_hash = hex::encode(prev_display);

    let mut merkle_bytes = [0u8; 32];
    merkle_bytes.copy_from_slice(&data[36..68]);
    let mut merkle_display = merkle_bytes;
    merkle_display.reverse();
    let merkle_root = hex::encode(merkle_display);

    let timestamp = u32::from_le_bytes([data[68], data[69], data[70], data[71]]);
    let bits = u32::from_le_bytes([data[72], data[73], data[74], data[75]]);
    let nonce = u32::from_le_bytes([data[76], data[77], data[78], data[79]]);

    // Block hash = SHA256d(80-byte header), reversed for display
    let hash = sha256d(&data[..80]);
    let mut hash_display = hash;
    hash_display.reverse();
    let block_hash = hex::encode(hash_display);

    Ok(BlockHeader {
        version,
        prev_block_hash,
        merkle_root,
        merkle_root_bytes: merkle_bytes,
        timestamp,
        bits,
        nonce,
        block_hash,
        block_hash_bytes: hash,
    })
}

/// Parse a rev*.dat undo file.
/// Returns undo data for each block in order.
/// Each block undo = Vec of CTxUndo (one per non-coinbase tx)
/// Each CTxUndo = Vec of UndoPrevout (one per input)
pub fn parse_rev_file(
    raw_data: &[u8],
    xor_key: &[u8; 8],
) -> Result<Vec<Vec<Vec<UndoPrevout>>>, AppError> {
    let mut data = raw_data.to_vec();
    xor_decode(&mut data, xor_key, 0);

    let mut pos = 0;
    let mut all_blocks = Vec::new();

    while pos < data.len() {
        if pos + 4 > data.len() {
            break;
        }

        // Skip zero padding
        if data[pos..pos + 4] == [0, 0, 0, 0] {
            break;
        }

        // Magic
        let magic = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        if magic != MAINNET_MAGIC {
            return Err(AppError::invalid_undo(format!(
                "Invalid magic at offset {}: {:08x}",
                pos, magic
            )));
        }
        pos += 4;

        // Size (4 bytes LE)
        if pos + 4 > data.len() {
            return Err(AppError::invalid_undo("Truncated undo file: missing size"));
        }
        let record_size =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        // record_size = CBlockUndo payload; followed by 32-byte checksum
        if pos + record_size + 32 > data.len() {
            return Err(AppError::invalid_undo(format!(
                "Undo record truncated at offset {}",
                pos
            )));
        }

        // The record format is: CBlockUndo(record_size bytes) + checksum(32 bytes)
        // The checksum is NOT included in record_size; it follows immediately after.
        let payload = &data[pos..pos + record_size];
        // (We skip checksum verification for now; could add later)

        let block_undo = parse_block_undo(payload)?;
        all_blocks.push(block_undo);

        // Advance past CBlockUndo + 32-byte checksum
        pos += record_size + 32;
    }

    Ok(all_blocks)
}

/// Parse a CBlockUndo payload (without magic/size/checksum wrapper).
/// Returns Vec of CTxUndo, each being Vec of UndoPrevout.
pub fn parse_block_undo(data: &[u8]) -> Result<Vec<Vec<UndoPrevout>>, AppError> {
    let mut pos = 0;

    // Number of CTxUndo entries (CompactSize)
    let (tx_count, cs) = read_compact_size(data, pos)?;
    pos += cs;

    let mut block_undo = Vec::with_capacity(tx_count as usize);

    for _tx_i in 0..tx_count {
        // Number of inputs (prevouts) in this tx (CompactSize)
        let (in_count, cs) = read_compact_size(data, pos)?;
        pos += cs;

        let mut tx_undo = Vec::with_capacity(in_count as usize);

        for _in_i in 0..in_count {
            // TxInUndoFormatter:
            // VARINT(nHeightAndCoinbase) = nHeight * 2 | fCoinBase
            let (height_and_cb, consumed) = read_varint(data, pos)?;
            pos += consumed;
            let n_height = height_and_cb >> 1;
            let _is_coinbase = (height_and_cb & 1) != 0;

            // If nHeight > 0, there's a dummy VarInt 0x00 for backward compat
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

/// Decompress a Bitcoin amount from undo data (CompressAmount inverse)
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

    // x * 10^e
    for _ in 0..e {
        x *= 10;
    }
    x
}

/// Read and decompress a script from undo data.
/// Returns (script_pubkey_bytes, bytes_consumed).
pub fn decompress_script(data: &[u8], pos: usize) -> Result<(Vec<u8>, usize), AppError> {
    // Read nSize as VarInt
    let (n_size, cs) = read_varint(data, pos)?;
    //let total_consumed_start = cs;

    match n_size {
        0 => {
            // P2PKH: 20-byte hash
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
            // Script: OP_PUSHBYTES_33 <prefix><x> OP_CHECKSIG
            let mut script = Vec::with_capacity(35);
            script.push(0x21); // push 33 bytes
            script.push(prefix);
            script.extend_from_slice(x_bytes);
            script.push(0xac); // OP_CHECKSIG
            Ok((script, cs + 32))
        }
        4 | 5 => {
            // P2PK uncompressed: stored as 32-byte x-coordinate, need to reconstruct 65-byte pubkey
            if pos + cs + 32 > data.len() {
                return Err(AppError::invalid_undo(
                    "Truncated P2PK uncompressed in undo",
                ));
            }
            let x_bytes = &data[pos + cs..pos + cs + 32];
            // The stored prefix indicates parity: 4 = even Y, 5 = odd Y
            // Reconstruct the compressed form first (same x), then decompress
            let compressed_prefix = if n_size == 4 { 0x02u8 } else { 0x03u8 };
            let mut compressed = Vec::with_capacity(33);
            compressed.push(compressed_prefix);
            compressed.extend_from_slice(x_bytes);
            // Decompress the pubkey to 65 bytes
            let uncompressed = decompress_pubkey(&compressed)?;
            // Script: OP_PUSHBYTES_65 <uncompressed 65 bytes> OP_CHECKSIG
            let mut script = Vec::with_capacity(67);
            script.push(0x41); // push 65 bytes
            script.extend_from_slice(&uncompressed);
            script.push(0xac); // OP_CHECKSIG
            Ok((script, cs + 32))
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

/// Decompress a 33-byte compressed public key to 65-byte uncompressed form.
fn decompress_pubkey(compressed: &[u8]) -> Result<Vec<u8>, AppError> {
    use secp256k1::PublicKey;
    let pk = PublicKey::from_slice(compressed)
        .map_err(|e| AppError::invalid_undo(format!("Invalid compressed pubkey: {}", e)))?;
    Ok(pk.serialize_uncompressed().to_vec())
}

