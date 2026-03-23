/// Output script types recognized by the analyzer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptType {
    P2PKH,
    P2SH,
    P2WPKH,
    P2WSH,
    P2TR,
    OpReturn,
    P2PK,
    Unknown,
}

impl ScriptType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptType::P2PKH => "p2pkh",
            ScriptType::P2SH => "p2sh",
            ScriptType::P2WPKH => "p2wpkh",
            ScriptType::P2WSH => "p2wsh",
            ScriptType::P2TR => "p2tr",
            ScriptType::OpReturn => "op_return",
            ScriptType::P2PK => "p2pk",
            ScriptType::Unknown => "unknown",
        }
    }

    /// Map to the JSON output categories (p2pk gets folded into unknown for the schema)
    pub fn json_key(&self) -> &'static str {
        match self {
            ScriptType::P2PKH => "p2pkh",
            ScriptType::P2SH => "p2sh",
            ScriptType::P2WPKH => "p2wpkh",
            ScriptType::P2WSH => "p2wsh",
            ScriptType::P2TR => "p2tr",
            ScriptType::OpReturn => "op_return",
            ScriptType::P2PK | ScriptType::Unknown => "unknown",
        }
    }
}

/// Classify an output script (scriptPubKey) into its type
pub fn classify_script(script: &[u8]) -> ScriptType {
    let s = bitcoin::Script::from_bytes(script);
    if s.is_p2pkh() {
        return ScriptType::P2PKH;
    }
    if s.is_p2sh() {
        return ScriptType::P2SH;
    }
    if s.is_p2wpkh() {
        return ScriptType::P2WPKH;
    }
    if s.is_p2wsh() {
        return ScriptType::P2WSH;
    }
    if s.is_p2tr() {
        return ScriptType::P2TR;
    }
    if s.is_op_return() {
        return ScriptType::OpReturn;
    }
    if s.is_p2pk() {
        return ScriptType::P2PK;
    }
    ScriptType::Unknown
}

/// Extract a "fingerprint" for address reuse detection.
/// Returns a hex-encoded identifier that uniquely identifies the address/script.
/// For standard types this is the hash payload; for others it's the full script hash.
pub fn script_fingerprint(script: &[u8]) -> String {
    let stype = classify_script(script);
    match stype {
        ScriptType::P2PKH => {
            // Extract the 20-byte hash160
            hex::encode(&script[3..23])
        }
        ScriptType::P2SH => hex::encode(&script[2..22]),
        ScriptType::P2WPKH => hex::encode(&script[2..22]),
        ScriptType::P2WSH => hex::encode(&script[2..34]),
        ScriptType::P2TR => hex::encode(&script[2..34]),
        _ => {
            // For unknown/OP_RETURN/P2PK, use the full script hex
            hex::encode(script)
        }
    }
}

/// Classify the OP_RETURN data by known protocols
pub fn classify_op_return(script: &[u8]) -> &'static str {
    if script.len() < 2 {
        return "empty";
    }
    // Skip the OP_RETURN opcode (0x6a)
    let payload = if script.len() > 1 {
        // There may be a push opcode after OP_RETURN
        let after_opreturn = &script[1..];
        if after_opreturn.is_empty() {
            return "empty";
        }
        // Check if next byte is a push length
        let push_len = after_opreturn[0] as usize;
        if push_len <= 0x4b && after_opreturn.len() > 1 {
            &after_opreturn[1..]
        } else {
            after_opreturn
        }
    } else {
        return "empty";
    };

    if payload.is_empty() {
        return "empty";
    }

    // Omni Layer: "omni" or 0x6f6d6e69
    if payload.len() >= 4 && &payload[..4] == b"omni" {
        return "omni_layer";
    }

    // OpenTimestamps: 0x4f5453 ("OTS")
    if payload.len() >= 3 && &payload[..3] == b"OTS" {
        return "opentimestamps";
    }

    // Stacks (Blockstack): 0x69643a (starts with "id:")
    if payload.len() >= 3 && &payload[..3] == b"id:" {
        return "stacks";
    }

    // RUNES protocol: OP_13 (0x5d) marker
    if script.len() >= 2 && script[1] == 0x5d {
        return "runes";
    }

    // Ordinals envelope detection is complex (done in witness), mark as generic
    // Counterparty: CNTRPRTY prefix
    if payload.len() >= 8 && &payload[..8] == b"CNTRPRTY" {
        return "counterparty";
    }

    // Veriblock: starts with specific magic bytes
    if payload.len() >= 2 && payload[0] == 0x00 && payload.len() > 40 {
        return "veriblock";
    }

    "data_carrier"
}
