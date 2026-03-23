use crate::forensics::block_parser::UndoPrevout;
use crate::forensics::script::{classify_script, script_fingerprint, ScriptType};
use crate::forensics::tx_parser::RawTx;
use crate::forensics::types::HeuristicResult;
use std::collections::{HashMap, HashSet};

pub mod address_reuse;
pub mod change_detection;
pub mod cioh;
pub mod coinjoin;
pub mod consolidation;
pub mod helpers;
pub mod op_return;
pub mod peeling_chain;
pub mod round_number;
pub mod self_transfer;

/// All heuristic IDs applied by the engine
pub const HEURISTIC_IDS: &[&str] = &[
    "cioh",
    "change_detection",
    "address_reuse",
    "coinjoin",
    "consolidation",
    "self_transfer",
    "peeling_chain",
    "op_return",
    "round_number_payment",
];

/// Context for analyzing a single transaction
pub struct TxContext<'a> {
    pub tx: &'a RawTx,
    /// Prevout data for each input (from undo). Empty for coinbase.
    pub prevouts: &'a [UndoPrevout],
    /// Whether this is a coinbase transaction
    pub is_coinbase: bool,
    /// Script fingerprints seen in earlier transactions in the same block.
    pub seen_fingerprints: &'a HashSet<String>,
    /// Legacy block-wide fingerprint set kept for compatibility with existing
    /// tests/callers. Temporal cross-tx reuse detection uses seen_fingerprints.
    pub block_fingerprints: &'a HashSet<String>,
}

/// Shared transaction-derived data used across heuristics.
pub struct TxSharedData {
    pub input_types: Vec<ScriptType>,
    pub output_types: Vec<ScriptType>,
    pub input_fingerprints: Vec<String>,
    pub output_fingerprints: Vec<String>,
    pub dominant_input_type: ScriptType,
    pub non_opreturn_output_indices: Vec<usize>,
    pub round_output_indices: Vec<usize>,
    pub output_precisions: Vec<u32>,
    pub total_input_value: u64,
    pub total_output_value: u64,
    pub input_values: Vec<u64>,
    pub output_values: Vec<u64>,
    pub output_value_counts: HashMap<u64, usize>,
    pub input_type_count: usize,
    pub output_type_count: usize,
    pub all_inputs_same_type: bool,
    pub all_inputs_segwit: bool,
    pub is_bip69_sorted: bool,
}

impl TxSharedData {
    pub fn new(ctx: &TxContext) -> Self {
        let input_types: Vec<ScriptType> = ctx
            .prevouts
            .iter()
            .map(|p| classify_script(&p.script_pubkey))
            .collect();
        let output_types: Vec<ScriptType> = ctx
            .tx
            .outputs
            .iter()
            .map(|o| classify_script(&o.script_pubkey))
            .collect();

        let input_fingerprints: Vec<String> = ctx
            .prevouts
            .iter()
            .map(|p| script_fingerprint(&p.script_pubkey))
            .collect();
        let output_fingerprints: Vec<String> = ctx
            .tx
            .outputs
            .iter()
            .map(|o| script_fingerprint(&o.script_pubkey))
            .collect();

        let dominant_input_type = helpers::get_dominant_type(&input_types);
        let non_opreturn_output_indices: Vec<usize> = output_types
            .iter()
            .enumerate()
            .filter(|(_, t)| **t != ScriptType::OpReturn)
            .map(|(i, _)| i)
            .collect();
        let round_output_indices: Vec<usize> = ctx
            .tx
            .outputs
            .iter()
            .enumerate()
            .filter(|(_, o)| helpers::is_round_amount(o.value_sats))
            .map(|(i, _)| i)
            .collect();
        let output_precisions: Vec<u32> = ctx
            .tx
            .outputs
            .iter()
            .map(|o| helpers::count_trailing_zeros(o.value_sats))
            .collect();

        let input_values: Vec<u64> = ctx.prevouts.iter().map(|p| p.value_sats).collect();
        let output_values: Vec<u64> = ctx
            .tx
            .outputs
            .iter()
            .enumerate()
            .filter(|(i, o)| output_types[*i] != ScriptType::OpReturn && o.value_sats > 0)
            .map(|(_, o)| o.value_sats)
            .collect();

        let input_type_count = input_types
            .iter()
            .filter(|t| **t != ScriptType::Unknown && **t != ScriptType::OpReturn)
            .collect::<HashSet<_>>()
            .len();
        let output_type_count = output_types
            .iter()
            .filter(|t| **t != ScriptType::Unknown && **t != ScriptType::OpReturn)
            .collect::<HashSet<_>>()
            .len();

        let all_inputs_same_type = input_types
            .iter()
            .filter(|t| **t != ScriptType::Unknown && **t != ScriptType::OpReturn)
            .all(|t| *t == dominant_input_type);
        let all_inputs_segwit = input_types
            .iter()
            .all(|t| *t == ScriptType::P2WPKH || *t == ScriptType::P2WSH || *t == ScriptType::P2TR);

        Self {
            total_input_value: input_values.iter().sum(),
            total_output_value: ctx.tx.outputs.iter().map(|o| o.value_sats).sum(),
            output_value_counts: helpers::value_frequency_map(&output_values),
            is_bip69_sorted: helpers::is_bip69_sorted(&ctx.tx.outputs),
            input_types,
            output_types,
            input_fingerprints,
            output_fingerprints,
            dominant_input_type,
            non_opreturn_output_indices,
            round_output_indices,
            output_precisions,
            input_values,
            output_values,
            input_type_count,
            output_type_count,
            all_inputs_same_type,
            all_inputs_segwit,
        }
    }
}

/// Result of all heuristics on a single transaction
pub struct TxAnalysis {
    pub heuristics: HashMap<String, HeuristicResult>,
    pub classification: String,
}

pub type OutPoint = (String, u32);
pub type SpendIndex = HashMap<OutPoint, Option<OutPoint>>;

pub fn analyze_transaction(ctx: &TxContext) -> TxAnalysis {
    analyze_transaction_in_block(ctx, &[])
}

pub fn analyze_transaction_in_block(ctx: &TxContext, block_transactions: &[RawTx]) -> TxAnalysis {
    analyze_transaction_with_spend_index(ctx, block_transactions, None)
}

pub fn analyze_transaction_with_spend_index(
    ctx: &TxContext,
    block_transactions: &[RawTx],
    spend_index: Option<&SpendIndex>,
) -> TxAnalysis {
    let mut heuristics = HashMap::new();

    if ctx.is_coinbase {
        // Coinbase transactions don't have meaningful heuristic results
        for &id in HEURISTIC_IDS {
            heuristics.insert(id.to_string(), HeuristicResult::not_detected());
        }
        return TxAnalysis {
            heuristics,
            classification: "unknown".to_string(),
        };
    }

    let shared = TxSharedData::new(ctx);

    heuristics.insert("cioh".to_string(), cioh::detect_cioh(ctx, &shared));

    heuristics.insert(
        "coinjoin".to_string(),
        coinjoin::detect_coinjoin(ctx, &shared),
    );

    heuristics.insert(
        "change_detection".to_string(),
        change_detection::detect_change(ctx, &shared),
    );

    heuristics.insert(
        "address_reuse".to_string(),
        address_reuse::detect_address_reuse(ctx, &shared),
    );

    heuristics.insert(
        "consolidation".to_string(),
        consolidation::detect_consolidation(ctx, &shared),
    );

    heuristics.insert(
        "self_transfer".to_string(),
        self_transfer::detect_self_transfer(ctx, &shared),
    );

    heuristics.insert(
        "peeling_chain".to_string(),
        peeling_chain::detect_peeling_chain(ctx, &shared, block_transactions, spend_index),
    );

    heuristics.insert(
        "op_return".to_string(),
        op_return::detect_op_return(ctx, &shared),
    );

    heuristics.insert(
        "round_number_payment".to_string(),
        round_number::detect_round_number(ctx, &shared),
    );

    // === Post-processing: cross-heuristic awareness ===
    // Heuristics can now influence each other's confidence.
    cross_heuristic_adjustments(&mut heuristics);

    // Classify transaction
    let classification = classify_transaction(ctx, &heuristics, &shared);

    TxAnalysis {
        heuristics,
        classification,
    }
}

/// Post-processing pass: let heuristic results influence each other.
///
/// This implements cross-heuristic awareness so that detections are
/// contextually consistent rather than independent.
fn cross_heuristic_adjustments(heuristics: &mut HashMap<String, HeuristicResult>) {
    // --- 1. CoinJoin detected → suppress CIOH ---
    // CoinJoin deliberately violates the common-input-ownership assumption.
    // If CoinJoin is detected (any confidence), reduce CIOH confidence
    // and add an explanatory note.
    let coinjoin_detected = heuristics
        .get("coinjoin")
        .map(|h| h.detected)
        .unwrap_or(false);
    let coinjoin_confidence = heuristics
        .get("coinjoin")
        .and_then(|h| h.confidence.clone())
        .unwrap_or_default();

    if coinjoin_detected {
        if let Some(cioh) = heuristics.get_mut("cioh") {
            if cioh.detected {
                // Reduce confidence based on CoinJoin confidence
                let penalty = match coinjoin_confidence.as_str() {
                    "high" => 0.35,
                    "medium" => 0.25,
                    _ => 0.15,
                };
                if let Some(ref mut score) = cioh.confidence_score {
                    *score = (*score - penalty).max(0.1);
                    cioh.confidence =
                        Some(helpers::confidence_label_from_score(*score).to_string());
                }
                let note = format!(
                    ". CIOH confidence reduced by {:.0}% due to {} CoinJoin detection",
                    penalty * 100.0,
                    coinjoin_confidence
                );
                if let Some(ref mut d) = cioh.details {
                    d.push_str(&note);
                }
            }
        }

        // CoinJoin also suppresses self-transfer (they look similar structurally)
        if let Some(st) = heuristics.get_mut("self_transfer") {
            if st.detected && coinjoin_confidence != "low" {
                if let Some(ref mut score) = st.confidence_score {
                    *score = (*score - 0.2).max(0.1);
                    st.confidence = Some(helpers::confidence_label_from_score(*score).to_string());
                }
                if let Some(ref mut d) = st.details {
                    d.push_str(
                        ". Self-transfer confidence reduced due to CoinJoin detection overlap",
                    );
                }
            }
        }
    }

    // --- 2. Consolidation detected → strengthen CIOH ---
    // Consolidation strongly implies single ownership of all inputs.
    let consolidation_detected = heuristics
        .get("consolidation")
        .map(|h| h.detected)
        .unwrap_or(false);
    let consolidation_high = heuristics
        .get("consolidation")
        .and_then(|h| h.confidence.as_deref().map(|c| c == "high"))
        .unwrap_or(false);

    if consolidation_detected && !coinjoin_detected {
        if let Some(cioh) = heuristics.get_mut("cioh") {
            if cioh.detected {
                let boost = if consolidation_high { 0.1 } else { 0.05 };
                if let Some(ref mut score) = cioh.confidence_score {
                    *score = (*score + boost).min(0.95);
                    cioh.confidence =
                        Some(helpers::confidence_label_from_score(*score).to_string());
                }
                if let Some(ref mut d) = cioh.details {
                    d.push_str(
                        ". CIOH strengthened by consolidation pattern (single-entity fan-in)",
                    );
                }
            }
        }
    }

    // --- 3. Address self-reuse → confirm change detection ---
    // If address_reuse detects self_reuse (input address reused in output),
    // and change_detection also found a candidate, the reused output is
    // almost certainly change.
    let self_reuse_detected = heuristics
        .get("address_reuse")
        .map(|h| {
            h.detected
                && h.reuse_type
                    .as_deref()
                    .map(|t| t == "self_reuse" || t == "mixed")
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    if self_reuse_detected {
        if let Some(change) = heuristics.get_mut("change_detection") {
            if change.detected {
                if let Some(ref mut score) = change.confidence_score {
                    *score = (*score + 0.1).min(0.95);
                    change.confidence =
                        Some(helpers::confidence_label_from_score(*score).to_string());
                }
                if let Some(ref mut d) = change.details {
                    d.push_str(". Change detection strengthened by address self-reuse (input address appears in output)");
                }
            }
        }
    }

    // --- 4. Self-transfer detected → suppress change detection relevance ---
    // In a self-transfer there is no "payment vs change" distinction.
    let self_transfer_detected = heuristics
        .get("self_transfer")
        .map(|h| h.detected)
        .unwrap_or(false);

    if self_transfer_detected && !coinjoin_detected {
        if let Some(change) = heuristics.get_mut("change_detection") {
            if change.detected {
                if let Some(ref mut score) = change.confidence_score {
                    *score = (*score - 0.15).max(0.1);
                    change.confidence =
                        Some(helpers::confidence_label_from_score(*score).to_string());
                }
                if let Some(ref mut d) = change.details {
                    d.push_str(". Change detection weakened: self-transfer detected (no meaningful payment/change distinction)");
                }
            }
        }
    }

    // --- 5. Peeling chain + change detection: mutual reinforcement ---
    let peeling_detected = heuristics
        .get("peeling_chain")
        .map(|h| h.detected)
        .unwrap_or(false);

    if peeling_detected {
        if let Some(change) = heuristics.get_mut("change_detection") {
            if change.detected {
                if let Some(ref mut score) = change.confidence_score {
                    *score = (*score + 0.05).min(0.95);
                    change.confidence =
                        Some(helpers::confidence_label_from_score(*score).to_string());
                }
                if let Some(ref mut d) = change.details {
                    d.push_str(". Reinforced by peeling chain pattern (large output is likely change/continuation)");
                }
            }
        }
    }
}

// Transaction Classification
fn classify_transaction(
    ctx: &TxContext,
    heuristics: &HashMap<String, HeuristicResult>,
    shared: &TxSharedData,
) -> String {
    use std::collections::HashSet;

    const MAX_SIMPLE_INPUTS: usize = 3;
    const MAX_SIMPLE_OUTPUTS: usize = 2;

    const MAX_BATCH_INPUTS: usize = 2;
    const MIN_BATCH_OUTPUTS: usize = 3;
    const LARGE_BATCH_OUTPUTS: usize = 5;

    let inputs = ctx.tx.inputs.len();
    let non_opreturn_outputs = shared.non_opreturn_output_indices.len();

    // Helper for medium+ confidence
    let medium_plus = |h: &HeuristicResult| {
        h.detected
            && h.confidence
                .as_deref()
                .map(|c| c == "high" || c == "medium")
                .unwrap_or(false)
    };

    // -------------------------
    // 1. CoinJoin (most distinct)
    // -------------------------
    if heuristics
        .get("coinjoin")
        .map(|h| medium_plus(h))
        .unwrap_or(false)
    {
        return "coinjoin".to_string();
    }

    // -------------------------
    // 2. Consolidation
    // -------------------------
    if heuristics
        .get("consolidation")
        .map(|h| medium_plus(h))
        .unwrap_or(false)
    {
        return "consolidation".to_string();
    }

    // -------------------------
    // 3. Self-transfer
    // -------------------------
    if heuristics
        .get("self_transfer")
        .map(|h| medium_plus(h))
        .unwrap_or(false)
    {
        return "self_transfer".to_string();
    }

    // -------------------------
    // 4. Batch payment
    // -------------------------

    // Typical exchange payout pattern
    if inputs <= MAX_BATCH_INPUTS && non_opreturn_outputs >= MIN_BATCH_OUTPUTS {
        return "batch_payment".to_string();
    }

    // Larger fan-out pattern
    if non_opreturn_outputs >= LARGE_BATCH_OUTPUTS {
        let output_types: HashSet<&ScriptType> = shared
            .output_types
            .iter()
            .filter(|t| **t != ScriptType::OpReturn)
            .collect();

        if output_types.len() >= 2 {
            return "batch_payment".to_string();
        }
    }

    // -------------------------
    // 5. Simple payment
    // -------------------------
    if inputs <= MAX_SIMPLE_INPUTS && non_opreturn_outputs <= MAX_SIMPLE_OUTPUTS {
        let has_change = heuristics
            .get("change_detection")
            .map(|h| h.detected)
            .unwrap_or(false);

        if has_change || non_opreturn_outputs == 1 {
            return "simple_payment".to_string();
        }
    }

    // -------------------------
    // 6. Fallback CoinJoin
    // -------------------------
    if heuristics
        .get("coinjoin")
        .map(|h| h.detected)
        .unwrap_or(false)
    {
        return "coinjoin".to_string();
    }

    // -------------------------
    // 7. Default fallback
    // -------------------------
    if non_opreturn_outputs <= MAX_SIMPLE_OUTPUTS {
        "simple_payment".to_string()
    } else {
        "unknown".to_string()
    }
}

// Re-export commonly used items from helpers
pub use helpers::build_block_fingerprints;
