/// BIP-68 / Bitcoin Core boundary: values >= this are UNIX timestamps, below are block heights.
const LOCKTIME_TIMESTAMP_BOUNDARY: u32 = 500_000_000;

/// RBF/Locktime computation result
#[derive(Debug, Clone)]
pub struct RbfLocktimeResult {
    pub n_sequence: u32,
    pub n_locktime: u32,
    pub rbf_signaling: bool,
    pub locktime_type: String,
}

/// Compute nSequence and nLockTime based on the fixture fields.

pub fn compute_rbf_locktime(
    rbf: Option<bool>,
    locktime: Option<u32>,
    current_height: Option<u32>,
) -> RbfLocktimeResult {
    let rbf_enabled = rbf.unwrap_or(false);

    let n_locktime = match (locktime, rbf_enabled, current_height) {
        (Some(lt), _, _) => lt,
        (None, true, Some(ch)) => ch, // anti-fee-sniping
        _ => 0,
    };

    let n_sequence = if rbf_enabled {
        0xFFFFFFFD_u32
    } else if n_locktime > 0 {
        0xFFFFFFFE_u32
    } else {
        0xFFFFFFFF_u32
    };

    // BIP-125: RBF is signaled when nSequence < 0xFFFFFFFE (i.e. <= 0xFFFFFFFD)
    let rbf_signaling = n_sequence < 0xFFFFFFFE;

    let locktime_type = if n_locktime == 0 {
        "none".to_string()
    } else if n_locktime < LOCKTIME_TIMESTAMP_BOUNDARY {
        "block_height".to_string()
    } else {
        "unix_timestamp".to_string()
    };

    RbfLocktimeResult {
        n_sequence,
        n_locktime,
        rbf_signaling,
        locktime_type,
    }
}
