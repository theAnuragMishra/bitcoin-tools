use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

/// SHA-256 single round
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

/// SHA-256d = SHA-256(SHA-256(data))
pub fn sha256d(data: &[u8]) -> [u8; 32] {
    sha256(&sha256(data))
}

/// HASH160 = RIPEMD-160(SHA-256(data))
pub fn hash160(data: &[u8]) -> [u8; 20] {
    let s = sha256(data);
    let mut h = Ripemd160::new();
    h.update(s);
    h.finalize().into()
}
