// Types matching the Rust JSON output schema

export interface RoundOutput {
  index: number;
  value_sats: number;
  btc_value: number;
}

export interface HeuristicResult {
  detected: boolean;
  likely_change_index?: number;
  method?: string;
  confidence?: string;
  details?: string;
  protocol?: string;
  equal_output_count?: number;
  equal_output_value?: number;
  input_count?: number;
  output_count?: number;
  reused_addresses?: string[];
  round_outputs?: RoundOutput[];
  confidence_score?: number;
  total_input_value?: number;
  total_output_value?: number;
  input_type_count?: number;
  dominant_input_type?: string;
  change_score?: number;
  secondary_change_index?: number;
  coinjoin_type?: string;
  denomination_count?: number;
  fan_in_ratio?: number;
  consolidated_value?: number;
  peel_ratio?: number;
  chain_continued?: boolean;
  chain_length?: number;
  data_size?: number;
  payload_hex?: string;
  protocols?: string[];
  reuse_type?: string;
  denomination_tiers?: string[];
  all_outputs_same_type?: boolean;
  privacy_score?: number;
}

export interface TxInputFlow {
  index: number;
  prev_txid?: string;
  prev_vout?: number;
  value_sats?: number;
  script_type?: string;
  is_coinbase?: boolean;
}

export interface TxOutputFlow {
  index: number;
  value_sats: number;
  script_type: string;
}

export interface TxOutput {
  txid: string;
  heuristics: Record<string, HeuristicResult>;
  classification: string;
  is_coinbase: boolean;
  input_count?: number;
  output_count?: number;
  total_input_value?: number;
  total_output_value?: number;
  total_fee?: number;
  // virtual size in vbytes (computed by the parser)
  vbytes: number;
  inputs?: TxInputFlow[];
  outputs?: TxOutputFlow[];
}

export interface FeeRateStats {
  min_sat_vb: number;
  max_sat_vb: number;
  median_sat_vb: number;
  mean_sat_vb: number;
}

export interface AnalysisSummary {
  total_transactions_analyzed: number;
  heuristics_applied: string[];
  flagged_transactions: number;
  script_type_distribution: Record<string, number>;
  fee_rate_stats: FeeRateStats;
}

export interface BlockOutput {
  block_hash: string;
  block_height: number;
  tx_count: number;
  analysis_summary: AnalysisSummary;
  transactions: TxOutput[];
  notable_transactions?: NotableTxSummary[];
}

export interface NotableHeuristicSummary {
  id: string;
  confidence?: string;
  method?: string;
  details?: string;
}

export interface NotableTxSummary {
  txid: string;
  classification: string;
  heuristics: NotableHeuristicSummary[];
}

export interface FileOutput {
  ok: boolean;
  mode: string;
  file: string;
  block_count: number;
  address_clusters: string[][];
  analysis_summary: AnalysisSummary;
  blocks: BlockOutput[];
}

export type Classification =
  | "simple_payment"
  | "consolidation"
  | "coinjoin"
  | "self_transfer"
  | "batch_payment"
  | "unknown";

export const CLASSIFICATION_COLORS: Record<string, string> = {
  simple_payment: "#3fb950",
  consolidation: "#58a6ff",
  coinjoin: "#bc8cff",
  self_transfer: "#d29922",
  batch_payment: "#39d2c0",
  unknown: "#8b949e",
};

export const CLASSIFICATION_LABELS: Record<string, string> = {
  simple_payment: "Simple Payment",
  consolidation: "Consolidation",
  coinjoin: "CoinJoin",
  self_transfer: "Self Transfer",
  batch_payment: "Batch Payment",
  unknown: "Unknown",
};

export const HEURISTIC_LABELS: Record<string, string> = {
  cioh: "Common Input Ownership",
  change_detection: "Change Detection",
  address_reuse: "Address Reuse",
  coinjoin: "CoinJoin Detection",
  consolidation: "Consolidation",
  self_transfer: "Self Transfer",
  peeling_chain: "Peeling Chain",
  op_return: "OP_RETURN Analysis",
  round_number_payment: "Round Number Payment",
};

export const HEURISTIC_COLORS: Record<string, string> = {
  cioh: "#f7931a",
  change_detection: "#58a6ff",
  address_reuse: "#f85149",
  coinjoin: "#bc8cff",
  consolidation: "#39d2c0",
  self_transfer: "#d29922",
  peeling_chain: "#f778ba",
  op_return: "#8b949e",
  round_number_payment: "#3fb950",
};

export const SCRIPT_TYPE_COLORS: Record<string, string> = {
  p2wpkh: "#58a6ff",
  p2tr: "#bc8cff",
  p2sh: "#39d2c0",
  p2pkh: "#d29922",
  p2wsh: "#f778ba",
  op_return: "#8b949e",
  unknown: "#484f58",
};
