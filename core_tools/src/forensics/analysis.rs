use std::collections::{HashMap, HashSet};

use crate::forensics::block_parser::{
    extract_bip34_height, parse_blk_file, parse_rev_file, ParsedBlock,
};
use crate::forensics::error::AppError;
use crate::forensics::heuristics;
use crate::forensics::heuristics::{OutPoint, SpendIndex, TxAnalysis, TxContext, HEURISTIC_IDS};

/// Decide whether a transaction should be flagged.
///
/// Flagging should be reserved for transactions with *strong, unusual* signals.
/// Common heuristics that fire on the majority of normal traffic (cioh,
/// change_detection, round_number_payment, cross-tx address_reuse, op_return)
/// do NOT cause flagging on their own — they are supportive signals that
/// strengthen other detections.
///
/// A transaction IS flagged when any of the following hold:
///   1. CoinJoin detected (any confidence)
///   2. Consolidation detected with medium+ confidence
///   3. Self-transfer detected with high confidence only
///   4. Peeling chain that continues in-block (chain_continued == true)
///   5. Three or more *independent* non-supportive heuristics fire simultaneously (unusual)
fn should_flag_transaction(analysis: &TxAnalysis) -> bool {
    // Strong-signal heuristics that flag independently
    if let Some(h) = analysis.heuristics.get("coinjoin") {
        if h.detected {
            return true;
        }
    }

    // Consolidation: medium+ confidence
    if let Some(h) = analysis.heuristics.get("consolidation") {
        if h.detected {
            if let Some(ref conf) = h.confidence {
                if conf == "high" || conf == "medium" {
                    return true;
                }
            }
        }
    }

    // Self-transfer: only high confidence (medium is too common after tightening)
    if let Some(h) = analysis.heuristics.get("self_transfer") {
        if h.detected {
            if let Some(ref conf) = h.confidence {
                if conf == "high" {
                    return true;
                }
            }
        }
    }

    // Peeling chain: only if it continues in-block (confirmed pattern).
    // A single 1-in/2-out tx with high ratio is just a normal payment with
    // large change — not worth flagging.
    if let Some(h) = analysis.heuristics.get("peeling_chain") {
        if h.detected {
            if let Some(true) = h.chain_continued {
                return true;
            }
        }
    }

    // Many simultaneous detections (excluding the "common" ones)
    // This catches unusual transactions where multiple independent signals fire.
    let supportive_only = [
        "cioh",
        "change_detection",
        "round_number_payment",
        "op_return",
    ];
    let strong_detections = analysis
        .heuristics
        .iter()
        .filter(|(id, h)| h.detected && !supportive_only.contains(&id.as_str()))
        .count();
    if strong_detections >= 3 {
        return true;
    }

    false
}
use crate::forensics::script::{classify_script, script_fingerprint};
use crate::forensics::types::*;

/// Analyze a block file and its corresponding undo file
pub fn analyze_block_file(
    blk_data: &[u8],
    rev_data: &[u8],
    xor_key: &[u8; 8],
    blk_filename: &str,
) -> Result<FileOutput, AppError> {
    // Parse both files
    let blk_blocks = parse_blk_file(blk_data, xor_key)?;
    let rev_blocks = parse_rev_file(rev_data, xor_key)?;

    if blk_blocks.len() != rev_blocks.len() {
        return Err(AppError::invalid_undo(format!(
            "blk file has {} blocks but rev file has {} undo records",
            blk_blocks.len(),
            rev_blocks.len()
        )));
    }

    // Pair each block with the matching undo record using the Bitcoin Core undo checksum.
    let mut undo_by_checksum = std::collections::HashMap::new();
    for undo_record in rev_blocks {
        undo_by_checksum.insert(undo_record.checksum, undo_record);
    }

    let mut paired: Vec<ParsedBlock> = Vec::new();
    for (header, transactions) in blk_blocks {
        let mut matching_key = None;
        for checksum in undo_by_checksum.keys().copied() {
            let undo_record = &undo_by_checksum[&checksum];
            let computed = compute_undo_checksum(&header.prev_block_hash, &undo_record.payload)?;
            if computed == checksum {
                matching_key = Some(checksum);
                break;
            }
        }

        let Some(checksum) = matching_key else {
            return Err(AppError::invalid_undo(format!(
                "Could not find matching undo record for block {}",
                header.block_hash
            )));
        };

        let undo_record = undo_by_checksum.remove(&checksum).ok_or_else(|| {
            AppError::invalid_undo(format!(
                "Missing matched undo record for block {}",
                header.block_hash
            ))
        })?;

        let height = if !transactions.is_empty() {
            extract_bip34_height(&transactions[0])?
        } else {
            0
        };
        paired.push(ParsedBlock {
            header,
            transactions,
            undo_prevouts: undo_record.prevouts,
            height,
        });
    }

    if !undo_by_checksum.is_empty() {
        return Err(AppError::invalid_undo(format!(
            "{} unmatched undo records remain after block pairing",
            undo_by_checksum.len()
        )));
    }

    // Analyze each block
    let mut block_outputs = Vec::new();
    let mut total_flagged: usize = 0;
    let mut total_tx: usize = 0;
    let mut file_script_dist: HashMap<String, usize> = HashMap::new();
    let mut file_heuristics: HashSet<String> = HashSet::new();
    let mut all_fee_rates: Vec<f64> = Vec::new();

    let spend_index = build_spend_index(&paired);
    let address_clusters = build_address_clusters(&paired, &spend_index);
    let mut seen_fingerprints: HashSet<String> = HashSet::new();

    for (block_idx, block) in paired.iter().enumerate() {
        let expected_undo_entries = block.transactions.len().saturating_sub(1);
        if block.undo_prevouts.len() != expected_undo_entries {
            return Err(AppError::invalid_undo(format!(
                "Block {} has {} transactions but {} undo entries (expected {})",
                block.header.block_hash,
                block.transactions.len(),
                block.undo_prevouts.len(),
                expected_undo_entries
            )));
        }

        let block_output = analyze_single_block(
            block,
            block_idx == 0,
            Some(&spend_index),
            &mut seen_fingerprints,
        )?;

        // Aggregate stats
        total_tx += block_output.tx_count;
        total_flagged += block_output.analysis_summary.flagged_transactions;

        for (k, v) in &block_output.analysis_summary.script_type_distribution {
            *file_script_dist.entry(k.clone()).or_insert(0) += v;
        }

        for hid in &block_output.analysis_summary.heuristics_applied {
            file_heuristics.insert(hid.clone());
        }

        // Collect fee rates from non-coinbase transactions
        for (tx_idx, tx) in block.transactions.iter().enumerate() {
            if !tx.is_coinbase() && tx_idx > 0 {
                if let Some(fee_rate) = compute_tx_fee_rate(tx, &block.undo_prevouts[tx_idx - 1]) {
                    all_fee_rates.push(fee_rate);
                }
            }
        }

        block_outputs.push(block_output);
    }

    // Compute file-level fee rate stats
    let file_fee_stats = compute_fee_rate_stats(&all_fee_rates);

    // File-level heuristics = union of all block heuristics (sorted for stable output)
    let mut file_heuristics: Vec<String> = file_heuristics.into_iter().collect();
    file_heuristics.sort();

    let analysis_summary = AnalysisSummary {
        total_transactions_analyzed: total_tx,
        heuristics_applied: file_heuristics,
        flagged_transactions: total_flagged,
        script_type_distribution: file_script_dist,
        fee_rate_stats: file_fee_stats,
    };

    Ok(FileOutput {
        ok: true,
        mode: "chain_analysis".to_string(),
        file: blk_filename.to_string(),
        block_count: block_outputs.len(),
        address_clusters,
        analysis_summary,
        blocks: block_outputs,
    })
}

fn compute_undo_checksum(prev_block_hash_hex: &str, payload: &[u8]) -> Result<[u8; 32], AppError> {
    let mut data = Vec::with_capacity(32 + payload.len());
    let mut prev = hex::decode(prev_block_hash_hex)
        .map_err(|e| AppError::invalid_block(format!("Invalid prev block hash hex: {}", e)))?;
    if prev.len() != 32 {
        return Err(AppError::invalid_block(format!(
            "Invalid prev block hash length: expected 32 bytes, got {}",
            prev.len()
        )));
    }
    prev.reverse();
    data.extend_from_slice(&prev);
    data.extend_from_slice(payload);
    Ok(crate::forensics::crypto::sha256d(&data))
}

fn build_spend_index(blocks: &[ParsedBlock]) -> SpendIndex {
    let mut tx_by_txid: HashMap<String, &crate::forensics::tx_parser::RawTx> = HashMap::new();
    for block in blocks {
        for tx in &block.transactions {
            tx_by_txid.insert(tx.txid.clone(), tx);
        }
    }

    let mut spends: SpendIndex = HashMap::new();

    for block in blocks {
        for tx in &block.transactions {
            for input in &tx.inputs {
                if input.txid_bytes == [0u8; 32] && input.vout == 0xFFFF_FFFF {
                    continue;
                }
                let next = continuation_outpoint_for(tx);
                spends.insert((input.txid.clone(), input.vout), next);
            }
        }
    }

    for tx in tx_by_txid.values() {
        for vout in 0..tx.outputs.len() {
            spends.entry((tx.txid.clone(), vout as u32)).or_insert(None);
        }
    }

    spends
}

fn continuation_outpoint_for(tx: &crate::forensics::tx_parser::RawTx) -> Option<OutPoint> {
    if tx.outputs.len() < 2 {
        return None;
    }

    let mut best: Option<(usize, u64)> = None;
    for (idx, output) in tx.outputs.iter().enumerate() {
        if output.value_sats == 0 {
            continue;
        }
        match best {
            Some((_, value)) if value >= output.value_sats => {}
            _ => {
                best = Some((idx, output.value_sats));
            }
        }
    }

    best.map(|(idx, _)| (tx.txid.clone(), idx as u32))
}

#[derive(Debug, Default)]
struct DisjointSet {
    parent: HashMap<String, String>,
    rank: HashMap<String, usize>,
}

impl DisjointSet {
    fn ensure(&mut self, x: &str) {
        if !self.parent.contains_key(x) {
            self.parent.insert(x.to_string(), x.to_string());
            self.rank.insert(x.to_string(), 0);
        }
    }

    fn find(&mut self, x: &str) -> String {
        self.ensure(x);
        let parent = self.parent.get(x).cloned().unwrap_or_else(|| x.to_string());
        if parent != x {
            let root = self.find(&parent);
            self.parent.insert(x.to_string(), root.clone());
            root
        } else {
            parent
        }
    }

    fn union(&mut self, a: &str, b: &str) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a == root_b {
            return;
        }

        let rank_a = *self.rank.get(&root_a).unwrap_or(&0);
        let rank_b = *self.rank.get(&root_b).unwrap_or(&0);

        if rank_a < rank_b {
            self.parent.insert(root_a, root_b);
        } else if rank_a > rank_b {
            self.parent.insert(root_b, root_a);
        } else {
            self.parent.insert(root_b.clone(), root_a.clone());
            self.rank.insert(root_a, rank_a + 1);
        }
    }

    fn clusters_of_min_size(&mut self, min_size: usize) -> Vec<Vec<String>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        let keys: Vec<String> = self.parent.keys().cloned().collect();
        for key in keys {
            let root = self.find(&key);
            groups.entry(root).or_default().push(key);
        }

        let mut clusters: Vec<Vec<String>> = groups
            .into_values()
            .filter(|g| g.len() >= min_size)
            .map(|mut g| {
                g.sort();
                g
            })
            .collect();
        clusters.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a[0].cmp(&b[0])));
        clusters
    }
}

fn build_address_clusters(blocks: &[ParsedBlock], spend_index: &SpendIndex) -> Vec<Vec<String>> {
    let mut dsu = DisjointSet::default();
    let empty_seen: HashSet<String> = HashSet::new();

    for block in blocks {
        for (tx_idx, tx) in block.transactions.iter().enumerate() {
            if tx_idx == 0 || tx.is_coinbase() {
                continue;
            }

            let prevouts = &block.undo_prevouts[tx_idx - 1];
            if prevouts.len() < 2 {
                continue;
            }

            let ctx = TxContext {
                tx,
                prevouts,
                is_coinbase: false,
                seen_fingerprints: &empty_seen,
                block_fingerprints: &empty_seen,
            };
            let analysis = crate::forensics::heuristics::analyze_transaction_with_spend_index(
                &ctx,
                &block.transactions,
                Some(spend_index),
            );

            let is_coinjoin = analysis
                .heuristics
                .get("coinjoin")
                .map(|h| h.detected)
                .unwrap_or(false);
            let cioh_detected = analysis
                .heuristics
                .get("cioh")
                .map(|h| h.detected)
                .unwrap_or(false);

            if is_coinjoin || !cioh_detected {
                continue;
            }

            let mut unique_inputs: Vec<String> = Vec::new();
            let mut seen = HashSet::new();
            for prevout in prevouts {
                let fp = script_fingerprint(&prevout.script_pubkey);
                if seen.insert(fp.clone()) {
                    unique_inputs.push(fp);
                }
            }

            if unique_inputs.len() < 2 {
                continue;
            }

            let anchor = &unique_inputs[0];
            for fp in unique_inputs.iter().skip(1) {
                dsu.union(anchor, fp);
            }
        }
    }

    dsu.clusters_of_min_size(3)
}

/// Analyze a single parsed block
/// to keep the output size manageable we only include transaction-level details for the first block
fn analyze_single_block(
    block: &ParsedBlock,
    include_transactions: bool,
    spend_index: Option<&SpendIndex>,
    seen_fingerprints: &mut HashSet<String>,
) -> Result<BlockOutput, AppError> {
    let tx_count = block.transactions.len();

    let mut tx_outputs = Vec::with_capacity(if include_transactions { tx_count } else { 0 });
    let mut flagged = 0;
    let mut script_dist: HashMap<String, usize> = HashMap::new();
    let mut fee_rates: Vec<f64> = Vec::new();
    let mut heuristic_findings: HashMap<String, usize> = HashMap::new();
    let mut classification_counts: HashMap<String, usize> = HashMap::new();
    let mut notable_transactions = Vec::new();

    for (i, tx) in block.transactions.iter().enumerate() {
        let is_coinbase = tx.is_coinbase();

        // Get prevouts for this transaction
        let prevouts = if is_coinbase {
            &[] as &[_]
        } else {
            // undo_prevouts integrity is validated before entering this function
            &block.undo_prevouts[i - 1]
        };

        // Classify output scripts and update distribution
        for output in &tx.outputs {
            let stype = classify_script(&output.script_pubkey);
            *script_dist.entry(stype.json_key().to_string()).or_insert(0) += 1;
        }

        // Compute fee rate for non-coinbase
        if !is_coinbase && !prevouts.is_empty() {
            if let Some(fee_rate) = compute_tx_fee_rate(tx, prevouts) {
                fee_rates.push(fee_rate);
            }
        }

        // Run heuristics
        let ctx = TxContext {
            tx,
            prevouts,
            is_coinbase,
            seen_fingerprints,
            block_fingerprints: seen_fingerprints,
        };
        let analysis = heuristics::analyze_transaction_with_spend_index(
            &ctx,
            &block.transactions,
            spend_index,
        );

        // Update the temporal set after analysis so only earlier transactions
        // influence cross-transaction reuse detection.
        for prevout in prevouts {
            seen_fingerprints.insert(script_fingerprint(&prevout.script_pubkey));
        }
        for output in &tx.outputs {
            seen_fingerprints.insert(script_fingerprint(&output.script_pubkey));
        }

        // Count flagged — only transactions with strong/unusual signals.
        // Common heuristics (cioh, change_detection, round_number_payment,
        // cross-tx address_reuse) fire on the majority of normal transactions
        // and should NOT trigger flagging on their own.  Only structurally
        // significant findings warrant a flag.
        let is_flagged = should_flag_transaction(&analysis);
        if is_flagged {
            flagged += 1;
        }

        for (hid, result) in &analysis.heuristics {
            if result.detected {
                *heuristic_findings.entry(hid.clone()).or_insert(0) += 1;
            }
        }

        *classification_counts
            .entry(analysis.classification.clone())
            .or_insert(0) += 1;

        let conditions = [
            analysis.classification == "coinjoin",
            analysis.classification == "consolidation",
            analysis.classification == "batch_payment",
            analysis.classification == "self_transfer"
                && analysis
                    .heuristics
                    .get("self_transfer")
                    .and_then(|h| h.confidence_score)
                    .unwrap_or_default()
                    >= 0.9,
            analysis.heuristics.values().filter(|h| h.detected).count() >= 4,
        ];

        let is_notable = conditions.iter().filter(|&&c| c).count() >= 2;

        if is_notable && notable_transactions.len() < 10 {
            let mut heuristics = Vec::new();
            for (hid, result) in &analysis.heuristics {
                if result.detected {
                    heuristics.push(NotableHeuristicSummary {
                        id: hid.clone(),
                        confidence: result.confidence.clone(),
                        method: result.method.clone(),
                        details: result.details.clone(),
                    });
                }
            }
            notable_transactions.push(NotableTxSummary {
                txid: tx.txid.clone(),
                classification: analysis.classification.clone(),
                heuristics,
            });
        }

        if include_transactions {
            let output_count = tx.outputs.len();
            let total_output_value: u64 = tx.outputs.iter().map(|o| o.value_sats).sum();
            let output_entries = tx
                .outputs
                .iter()
                .enumerate()
                .map(|(idx, out)| TxOutputFlow {
                    index: idx,
                    value_sats: out.value_sats,
                    script_type: classify_script(&out.script_pubkey).as_str().to_string(),
                })
                .collect::<Vec<_>>();

            let input_entries = if is_coinbase {
                Some(vec![TxInputFlow {
                    index: 0,
                    prev_txid: None,
                    prev_vout: None,
                    value_sats: None,
                    script_type: None,
                    is_coinbase: true,
                }])
            } else {
                Some(
                    tx.inputs
                        .iter()
                        .enumerate()
                        .map(|(idx, inp)| {
                            let prev = prevouts.get(idx);
                            TxInputFlow {
                                index: idx,
                                prev_txid: Some(inp.txid.clone()),
                                prev_vout: Some(inp.vout),
                                value_sats: prev.map(|p| p.value_sats),
                                script_type: prev.map(|p| {
                                    classify_script(&p.script_pubkey).as_str().to_string()
                                }),
                                is_coinbase: false,
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            };

            let (input_count, total_input_value, total_fee) = if is_coinbase {
                (None, None, None)
            } else {
                let in_total: u64 = prevouts.iter().map(|p| p.value_sats).sum();
                (
                    Some(tx.inputs.len()),
                    Some(in_total),
                    Some(in_total.saturating_sub(total_output_value)),
                )
            };

            tx_outputs.push(TxOutput {
                txid: tx.txid.clone(),
                heuristics: analysis.heuristics,
                classification: analysis.classification,
                is_coinbase,
                input_count,
                output_count: Some(output_count),
                total_input_value,
                total_output_value: Some(total_output_value),
                total_fee,
                vbytes: tx.vbytes,
                inputs: input_entries,
                outputs: Some(output_entries),
            });
        }
    }

    let fee_stats = compute_fee_rate_stats(&fee_rates);

    let analysis_summary = AnalysisSummary {
        total_transactions_analyzed: tx_count,
        heuristics_applied: HEURISTIC_IDS.iter().map(|s| s.to_string()).collect(),
        flagged_transactions: flagged,
        script_type_distribution: script_dist,
        fee_rate_stats: fee_stats,
    };

    Ok(BlockOutput {
        block_hash: block.header.block_hash.clone(),
        block_height: block.height,
        tx_count,
        analysis_summary,
        transactions: if include_transactions {
            tx_outputs
        } else {
            Vec::new()
        },
        block_timestamp: block.header.timestamp,
        heuristic_findings,
        classification_counts,
        notable_transactions,
    })
}

fn compute_tx_fee_rate(
    tx: &crate::forensics::tx_parser::RawTx,
    prevouts: &[crate::forensics::block_parser::UndoPrevout],
) -> Option<f64> {
    if prevouts.is_empty() || tx.vbytes == 0 {
        return None;
    }

    let input_total: u64 = prevouts.iter().map(|p| p.value_sats).sum();
    let output_total: u64 = tx.outputs.iter().map(|o| o.value_sats).sum();

    if input_total < output_total {
        return None;
    }

    Some((input_total - output_total) as f64 / tx.vbytes as f64)
}

/// Compute fee rate statistics from a list of fee rates
fn compute_fee_rate_stats(rates: &[f64]) -> FeeRateStats {
    if rates.is_empty() {
        return FeeRateStats {
            min_sat_vb: 0.0,
            max_sat_vb: 0.0,
            median_sat_vb: 0.0,
            mean_sat_vb: 0.0,
        };
    }

    let mut sorted = rates.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;

    let median = if sorted.len().is_multiple_of(2) {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    // Round to 1 decimal place for cleaner output
    FeeRateStats {
        min_sat_vb: (min * 10.0).round() / 10.0,
        max_sat_vb: (max * 10.0).round() / 10.0,
        median_sat_vb: (median * 10.0).round() / 10.0,
        mean_sat_vb: (mean * 10.0).round() / 10.0,
    }
}
