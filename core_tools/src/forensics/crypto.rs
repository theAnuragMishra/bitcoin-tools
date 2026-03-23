use bitcoin::hashes::{hash160 as bhash160, sha256 as bsha256, sha256d as bsha256d, Hash};

/// SHA-256 single round
pub fn sha256(data: &[u8]) -> [u8; 32] {
    bsha256::Hash::hash(data).to_byte_array()
}

/// SHA-256d = SHA-256(SHA-256(data))
pub fn sha256d(data: &[u8]) -> [u8; 32] {
    bsha256d::Hash::hash(data).to_byte_array()
}

/// HASH160 = RIPEMD-160(SHA-256(data))
pub fn hash160(data: &[u8]) -> [u8; 20] {
    bhash160::Hash::hash(data).to_byte_array()
}
