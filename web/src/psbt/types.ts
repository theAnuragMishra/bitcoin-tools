export interface Warning {
  code: string;
  message: string;
}

export interface SelectedInput {
  txid: string;
  vout: number;
  value_sats: number;
  script_pubkey_hex: string;
  script_type: string;
  address?: string;
}

export interface Output {
  script_pubkey_hex: string;
  script_type: string;
  value_sats: number;
  is_change: boolean;
  address?: string;
}

export interface BuildReport {
  ok: boolean;
  network: string;
  strategy: string;
  selected_inputs: SelectedInput[];
  outputs: Output[];
  change_index: number | null;
  fee_sats: number;
  fee_rate_sat_vb: number;
  vbytes: number;
  rbf_signaling: boolean;
  locktime: number;
  locktime_type: string;
  psbt_base64: string;
  //signed_psbt_base64?: string;
  // final_tx_hex?: string;
  watch_only_descriptors: {
    inputs: string[];
    outputs: string[];
  };
  privacy: {
    score: number;
    risk_level: string;
    factors: string[];
  };
  warnings: Warning[];
}

export interface ErrorReport {
  ok: false;
  error: { code: string; message: string };
}

export type ApiResponse = BuildReport | ErrorReport;
