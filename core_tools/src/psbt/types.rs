use serde::{Deserialize, Serialize};

/// Supported script types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScriptType {
    #[serde(rename = "p2pkh")]
    P2pkh,
    #[serde(rename = "p2wpkh")]
    P2wpkh,
    #[serde(rename = "p2sh-p2wpkh")]
    P2shP2wpkh,
    #[serde(rename = "p2sh-p2wsh")]
    P2shP2wsh,
    #[serde(rename = "op_return")]
    OpReturn,
    #[serde(rename = "p2tr")]
    P2tr,
    #[serde(rename = "p2sh")]
    P2sh,
    #[serde(rename = "p2wsh")]
    P2wsh,
}

impl ScriptType {
    /// Whether this input type uses segwit witness data
    pub fn is_segwit(&self) -> bool {
        matches!(
            self,
            ScriptType::P2wpkh
                | ScriptType::P2shP2wpkh
                | ScriptType::P2shP2wsh
                | ScriptType::P2tr
                | ScriptType::P2wsh
        )
    }
}

impl std::fmt::Display for ScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptType::P2pkh => write!(f, "p2pkh"),
            ScriptType::P2wpkh => write!(f, "p2wpkh"),
            ScriptType::P2shP2wpkh => write!(f, "p2sh-p2wpkh"),
            ScriptType::P2shP2wsh => write!(f, "p2sh-p2wsh"),
            ScriptType::OpReturn => write!(f, "op_return"),
            ScriptType::P2tr => write!(f, "p2tr"),
            ScriptType::P2sh => write!(f, "p2sh"),
            ScriptType::P2wsh => write!(f, "p2wsh"),
        }
    }
}

/// A UTXO from the fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub script_type: ScriptType,
    #[serde(default)]
    pub address: Option<String>,
}

/// A payment output from the fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    #[serde(default)]
    pub address: Option<String>,
    pub script_pubkey_hex: String,
    pub script_type: ScriptType,
    pub value_sats: u64,
}

/// Change template from the fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTemplate {
    #[serde(default)]
    pub address: Option<String>,
    pub script_pubkey_hex: String,
    pub script_type: ScriptType,
}

/// Policy constraints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Policy {
    #[serde(default)]
    pub max_inputs: Option<u32>,
}

/// The fixture input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub network: String,
    pub utxos: Vec<Utxo>,
    pub payments: Vec<Payment>,
    pub change: ChangeTemplate,
    pub fee_rate_sat_vb: f64,
    #[serde(default)]
    pub rbf: Option<bool>,
    #[serde(default)]
    pub locktime: Option<u32>,
    #[serde(default)]
    pub current_height: Option<u32>,
    #[serde(default)]
    pub policy: Option<Policy>,
    // #[serde(default)]
    // pub test_signing_wifs: Option<Vec<String>>,
}

/// Watch-only descriptor export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchOnlyDescriptors {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

/// Privacy meter details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyReport {
    pub score: u8,
    pub risk_level: String,
    pub factors: Vec<String>,
}

/// An output in the report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOutput {
    pub n: u32,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub script_type: ScriptType,
    pub address: Option<String>,
    pub is_change: bool,
}

/// A selected input in the report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportInput {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub script_type: ScriptType,
    pub address: Option<String>,
}

/// A warning entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub code: String,
}

/// The error detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

/// The JSON report (success case)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub ok: bool,
    pub network: String,
    pub strategy: String,
    pub selected_inputs: Vec<ReportInput>,
    pub outputs: Vec<ReportOutput>,
    pub change_index: Option<u32>,
    pub fee_sats: u64,
    pub fee_rate_sat_vb: f64,
    pub vbytes: u64,
    pub rbf_signaling: bool,
    pub locktime: u32,
    pub locktime_type: String,
    pub psbt_base64: String,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub signed_psbt_base64: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub final_tx_hex: Option<String>,
    pub watch_only_descriptors: WatchOnlyDescriptors,
    pub privacy: PrivacyReport,
    pub warnings: Vec<Warning>,
}

/// The JSON report (error case)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    pub ok: bool,
    pub error: ErrorDetail,
}

/// Builder error
#[derive(Debug, Clone)]
pub struct BuilderError {
    pub code: String,
    pub message: String,
}

impl BuilderError {
    pub fn new(code: &str, message: &str) -> Self {
        BuilderError {
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    pub fn to_error_report(&self) -> ErrorReport {
        ErrorReport {
            ok: false,
            error: ErrorDetail {
                code: self.code.clone(),
                message: self.message.clone(),
            },
        }
    }
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BuilderError {}
