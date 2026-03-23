use crate::visualiser::crypto::hash::sha256d;

/// Compute the Bitcoin Merkle root from a list of txids.
/// txids are in internal byte order (32 bytes each, little-endian as stored).
/// Returns the merkle root in the same internal byte order.
pub fn compute_merkle_root(txids: &[[u8; 32]]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }

    let mut layer: Vec<[u8; 32]> = txids.to_vec();

    while layer.len() > 1 {
        // If odd number, duplicate the last element
        if layer.len() % 2 == 1 {
            let last = *layer.last().unwrap();
            layer.push(last);
        }

        let mut next = Vec::with_capacity(layer.len() / 2);
        for chunk in layer.chunks_exact(2) {
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&chunk[0]);
            combined[32..].copy_from_slice(&chunk[1]);
            next.push(sha256d(&combined));
        }
        layer = next;
    }

    layer[0]
}
