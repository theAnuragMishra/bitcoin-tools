use serde::Serialize;
use std::collections::HashMap;

/// Top-level JSON output for a block file
#[derive(Debug, Serialize)]
pub struct FileOutput {
    pub ok: bool,
    pub mode: String,
    pub file: String,
    pub block_count: usize,
    pub address_clusters: Vec<Vec<String>>,
    pub analysis_summary: AnalysisSummary,
    pub blocks: Vec<BlockOutput>,
}

/// Analysis summary (shared structure for file-level and block-level)
#[derive(Debug, Serialize, Clone)]
pub struct AnalysisSummary {
    pub total_transactions_analyzed: usize,
    pub heuristics_applied: Vec<String>,
    pub flagged_transactions: usize,
    pub script_type_distribution: HashMap<String, usize>,
    pub fee_rate_stats: FeeRateStats,
}

/// Fee rate statistics
#[derive(Debug, Serialize, Clone)]
pub struct FeeRateStats {
    pub min_sat_vb: f64,
    pub max_sat_vb: f64,
    pub median_sat_vb: f64,
    pub mean_sat_vb: f64,
}

/// Per-block output
#[derive(Debug, Serialize)]
pub struct BlockOutput {
    pub block_hash: String,
    pub block_height: u64,
    pub tx_count: usize,
    pub analysis_summary: AnalysisSummary,
    pub transactions: Vec<TxOutput>,
    #[serde(skip_serializing)]
    pub block_timestamp: u32,
    #[serde(skip_serializing)]
    pub heuristic_findings: HashMap<String, usize>,
    #[serde(skip_serializing)]
    pub classification_counts: HashMap<String, usize>,
    pub notable_transactions: Vec<NotableTxSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotableTxSummary {
    pub txid: String,
    pub classification: String,
    pub heuristics: Vec<NotableHeuristicSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotableHeuristicSummary {
    pub id: String,
    pub confidence: Option<String>,
    pub method: Option<String>,
    pub details: Option<String>,
}

/// Per-transaction output
#[derive(Debug, Serialize)]
pub struct TxOutput {
    pub txid: String,
    pub heuristics: HashMap<String, HeuristicResult>,
    pub classification: String,
    pub is_coinbase: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_input_value: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_value: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_fee: Option<u64>,
    /// Virtual size in vbytes (BIP141 vsize).
    pub vbytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<TxInputFlow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<TxOutputFlow>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TxInputFlow {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_txid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_vout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_sats: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_type: Option<String>,
    #[serde(default)]
    pub is_coinbase: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct TxOutputFlow {
    pub index: usize,
    pub value_sats: u64,
    pub script_type: String,
}

/// Result of a single heuristic on a transaction
#[derive(Debug, Serialize, Clone)]
pub struct HeuristicResult {
    pub detected: bool,
    /// Index of output likely to be change (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub likely_change_index: Option<usize>,
    /// Name of heuristic method used (for context in results)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Confidence level (e.g. "low", "medium", "high") for detected heuristics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    /// Additional details or notes about the heuristic result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Detected protocol (for protocol-specific heuristics)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// For heuristics that count equal outputs (e.g. CoinJoin), number of outputs with same value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal_output_count: Option<usize>,
    /// For heuristics that identify equal outputs, the value of those outputs in satoshis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal_output_value: Option<u64>,
    /// Number of inputs in the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_count: Option<usize>,
    /// Number of outputs in the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_count: Option<usize>,
    /// For address reuse heuristics, list of reused address fingerprints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reused_addresses: Option<Vec<String>>,
    /// Details of outputs matching round denominations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_outputs: Option<Vec<RoundOutput>>,
    /// Numeric confidence score (0.0 - 1.0) for probabilistic assessment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<f64>,
    /// Total input value in satoshis (for value-aware heuristics)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_input_value: Option<u64>,
    /// Total output value in satoshis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_value: Option<u64>,
    /// Total fee in satoshis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_fee: Option<u64>,
    /// Number of distinct script types across inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type_count: Option<usize>,
    /// The dominant input script type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dominant_input_type: Option<String>,
    /// Change detection: score breakdown for transparency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_score: Option<i32>,
    /// Change detection: secondary candidate index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_change_index: Option<usize>,
    /// CoinJoin: detected sub-protocol signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coinjoin_type: Option<String>,
    /// CoinJoin: number of distinct equal-value denominations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denomination_count: Option<usize>,
    /// Consolidation: input-to-output ratio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fan_in_ratio: Option<f64>,
    /// Consolidation: total value consolidated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consolidated_value: Option<u64>,
    /// Peeling chain: ratio of large to small output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peel_ratio: Option<f64>,
    /// Peeling chain: whether a continuation was found in-block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_continued: Option<bool>,
    /// Peeling chain: length of chain detected within the block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_length: Option<usize>,
    /// OP_RETURN: data size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_size: Option<usize>,
    /// OP_RETURN: hex-encoded payload (capped)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_hex: Option<String>,
    /// OP_RETURN: all detected protocols (for multi-output)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocols: Option<Vec<String>>,
    /// Address reuse: type of reuse detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reuse_type: Option<String>,
    /// Round number: denomination tier (btc, mbtc, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denomination_tiers: Option<Vec<String>>,
    /// Self-transfer: whether all outputs are to same-type addresses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_outputs_same_type: Option<bool>,
    /// Privacy score (0-100, lower = less private)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_score: Option<u8>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RoundOutput {
    pub index: usize,
    pub value_sats: u64,
    pub btc_value: f64,
}

impl HeuristicResult {
    pub fn not_detected() -> Self {
        Self {
            detected: false,
            likely_change_index: None,
            method: None,
            confidence: None,
            details: None,
            protocol: None,
            equal_output_count: None,
            equal_output_value: None,
            input_count: None,
            output_count: None,
            reused_addresses: None,
            round_outputs: None,
            confidence_score: None,
            total_input_value: None,
            total_output_value: None,
            total_fee: None,
            input_type_count: None,
            dominant_input_type: None,
            change_score: None,
            secondary_change_index: None,
            coinjoin_type: None,
            denomination_count: None,
            fan_in_ratio: None,
            consolidated_value: None,
            peel_ratio: None,
            chain_continued: None,
            chain_length: None,
            data_size: None,
            payload_hex: None,
            protocols: None,
            reuse_type: None,
            denomination_tiers: None,
            all_outputs_same_type: None,
            privacy_score: None,
        }
    }

    pub fn detected() -> Self {
        Self {
            detected: true,
            likely_change_index: None,
            method: None,
            confidence: None,
            details: None,
            protocol: None,
            equal_output_count: None,
            equal_output_value: None,
            input_count: None,
            output_count: None,
            reused_addresses: None,
            round_outputs: None,
            confidence_score: None,
            total_input_value: None,
            total_output_value: None,
            total_fee: None,
            input_type_count: None,
            dominant_input_type: None,
            change_score: None,
            secondary_change_index: None,
            coinjoin_type: None,
            denomination_count: None,
            fan_in_ratio: None,
            consolidated_value: None,
            peel_ratio: None,
            chain_continued: None,
            chain_length: None,
            data_size: None,
            payload_hex: None,
            protocols: None,
            reuse_type: None,
            denomination_tiers: None,
            all_outputs_same_type: None,
            privacy_score: None,
        }
    }
}

/// Error output format
#[derive(Debug, Serialize)]
pub struct ErrorOutput {
    pub ok: bool,
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}
