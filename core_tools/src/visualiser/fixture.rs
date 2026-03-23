use crate::visualiser::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A prevout entry from the fixture file
#[derive(Debug, Clone, Deserialize)]
pub struct PrevoutEntry {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
}

/// The fixture file format for single-transaction mode
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub network: String,
    pub raw_tx: String,
    pub prevouts: Vec<PrevoutEntry>,
}

impl Fixture {
    pub fn from_json(json: &str) -> Result<Self, AppError> {
        serde_json::from_str(json)
            .map_err(|e| AppError::invalid_fixture(format!("Failed to parse fixture JSON: {}", e)))
    }

    /// Build a lookup map: (txid_lowercase, vout) -> PrevoutEntry
    /// Errors on duplicates.
    pub fn prevout_map(&self) -> Result<HashMap<(String, u32), &PrevoutEntry>, AppError> {
        let mut map = HashMap::new();
        for p in &self.prevouts {
            let key = (p.txid.to_lowercase(), p.vout);
            if map.insert(key.clone(), p).is_some() {
                return Err(AppError::duplicate_prevout(&p.txid, p.vout));
            }
        }
        Ok(map)
    }
}
