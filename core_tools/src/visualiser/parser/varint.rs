use crate::visualiser::error::AppError;

/// Read a Bitcoin CompactSize (variable-length integer) from `data` starting at `offset`.
/// Returns (value, bytes_consumed).
pub fn read_compact_size(data: &[u8], offset: usize) -> Result<(u64, usize), AppError> {
    if offset >= data.len() {
        return Err(AppError::invalid_tx(
            "Unexpected end of data reading compact size",
        ));
    }
    let first = data[offset];
    match first {
        0x00..=0xfc => Ok((first as u64, 1)),
        0xfd => {
            if offset + 3 > data.len() {
                return Err(AppError::invalid_tx("Truncated compact size (0xfd)"));
            }
            let val = u16::from_le_bytes([data[offset + 1], data[offset + 2]]) as u64;
            Ok((val, 3))
        }
        0xfe => {
            if offset + 5 > data.len() {
                return Err(AppError::invalid_tx("Truncated compact size (0xfe)"));
            }
            let val = u32::from_le_bytes([
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
            ]) as u64;
            Ok((val, 5))
        }
        0xff => {
            if offset + 9 > data.len() {
                return Err(AppError::invalid_tx("Truncated compact size (0xff)"));
            }
            let val = u64::from_le_bytes([
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
                data[offset + 8],
            ]);
            Ok((val, 9))
        }
    }
}

/// Read a Bitcoin VarInt (base-128, MSB-first with continuation bit).
/// Used in undo data (rev*.dat), NOT in transactions.
/// Returns (value, bytes_consumed).
pub fn read_varint(data: &[u8], offset: usize) -> Result<(u64, usize), AppError> {
    let mut n: u64 = 0;
    let mut pos = offset;
    loop {
        if pos >= data.len() {
            return Err(AppError::invalid_undo("Truncated VarInt"));
        }
        let byte = data[pos];
        pos += 1;
        // The encoding uses MSB as continuation bit; and a -1 trick:
        // n = (n << 7) | (byte & 0x7f), then if MSB not set, done
        // But Bitcoin's VarInt also does n++ between iterations so we can't have leading zeros
        n = (n << 7) | ((byte & 0x7f) as u64);
        if byte & 0x80 == 0 {
            // No more bytes
            return Ok((n, pos - offset));
        }
        // More bytes follow; apply the -1 trick to prevent leading zeros
        n += 1;
    }
}
