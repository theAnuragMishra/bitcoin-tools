use crate::forensics::block_parser::UndoPrevout;
use crate::forensics::script::script_fingerprint;
use crate::forensics::script::ScriptType;
use crate::forensics::tx_parser::{RawTx, TxOutput};
use std::collections::{HashMap, HashSet};

/// Get the most common script type from a list
pub fn get_dominant_type(types: &[ScriptType]) -> ScriptType {
    let mut counts: HashMap<ScriptType, usize> = HashMap::new();
    for t in types {
        if *t != ScriptType::Unknown && *t != ScriptType::OpReturn {
            *counts.entry(*t).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(t, _)| t)
        .unwrap_or(ScriptType::Unknown)
}

/// Check if an amount in satoshis is a "round" BTC value.
///
/// Only values that genuinely suggest human-chosen denominations are
/// considered "round".
/// the minimum granularity is 10,000 sats (0.0001 BTC / 0.1 mBTC).
pub fn is_round_amount(sats: u64) -> bool {
    if sats == 0 {
        return false;
    }

    // Any multiple of 100,000 sats (0.001 BTC) is considered round.
    // This captures: 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0 BTC, etc.
    if sats.is_multiple_of(100_000) {
        return true;
    }

    // Sats-denominated: multiples of 10,000 (0.0001 BTC)
    if sats.is_multiple_of(10_000) {
        return true;
    }

    false
}

/// Build a set of all script fingerprints in a block (for cross-tx address reuse detection)
pub fn build_block_fingerprints(
    transactions: &[RawTx],
    undo_data: &[Vec<UndoPrevout>],
) -> HashSet<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut reused: HashSet<String> = HashSet::new();

    // Collect fingerprints from outputs of all transactions
    for tx in transactions {
        for output in &tx.outputs {
            let fp = script_fingerprint(&output.script_pubkey);
            let count = seen.entry(fp.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                reused.insert(fp);
            }
        }
    }

    // Collect fingerprints from inputs (prevouts)
    for tx_undo in undo_data {
        for prevout in tx_undo {
            let fp = script_fingerprint(&prevout.script_pubkey);
            let count = seen.entry(fp.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                reused.insert(fp);
            }
        }
    }

    reused
}

/// Count trailing zeros in a satoshi value (measure of "roundness")
pub fn count_trailing_zeros(value: u64) -> u32 {
    if value == 0 {
        return 0;
    }
    let mut v = value;
    let mut count = 0u32;
    while v.is_multiple_of(10) {
        count += 1;
        v /= 10;
    }
    count
}

/// Check if outputs are sorted in BIP-69 lexicographic order.
/// BIP-69: outputs sorted by (value ascending, then scriptPubKey lexicographic)
pub fn is_bip69_sorted(outputs: &[TxOutput]) -> bool {
    if outputs.len() <= 1 {
        return true;
    }
    for i in 1..outputs.len() {
        let prev = &outputs[i - 1];
        let curr = &outputs[i];
        if prev.value_sats > curr.value_sats {
            return false;
        }
        if prev.value_sats == curr.value_sats && prev.script_pubkey > curr.script_pubkey {
            return false;
        }
    }
    true
}

/// Build a frequency map for integer values.
pub fn value_frequency_map(values: &[u64]) -> HashMap<u64, usize> {
    let mut counts = HashMap::new();
    for &value in values {
        *counts.entry(value).or_insert(0usize) += 1;
    }
    counts
}

/// Compute the coefficient of variation (standard deviation / mean).
pub fn coefficient_of_variation(values: &[u64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }

    let mean = values.iter().sum::<u64>() as f64 / values.len() as f64;
    if mean <= 0.0 {
        return None;
    }

    let variance = values
        .iter()
        .map(|&value| {
            let diff = value as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;

    Some(variance.sqrt() / mean)
}

/// Convert a numeric confidence score (0.0-1.0) to a stable label.
pub fn confidence_label_from_score(score: f64) -> &'static str {
    if score >= 0.75 {
        "high"
    } else if score >= 0.45 {
        "medium"
    } else {
        "low"
    }
}
