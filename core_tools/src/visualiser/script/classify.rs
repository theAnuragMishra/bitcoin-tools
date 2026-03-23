/// Output script types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputScriptType {
    P2PKH,
    P2SH,
    P2WPKH,
    P2WSH,
    P2TR,
    OpReturn,
    Unknown,
}

impl OutputScriptType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputScriptType::P2PKH => "p2pkh",
            OutputScriptType::P2SH => "p2sh",
            OutputScriptType::P2WPKH => "p2wpkh",
            OutputScriptType::P2WSH => "p2wsh",
            OutputScriptType::P2TR => "p2tr",
            OutputScriptType::OpReturn => "op_return",
            OutputScriptType::Unknown => "unknown",
        }
    }
}

/// Input script types (spend types)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputScriptType {
    P2PKH,
    P2SHP2WPKH,
    P2SHP2WSH,
    P2WPKH,
    P2WSH,
    P2TRKeyPath,
    P2TRScriptPath,
    Unknown,
}

impl InputScriptType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InputScriptType::P2PKH => "p2pkh",
            InputScriptType::P2SHP2WPKH => "p2sh-p2wpkh",
            InputScriptType::P2SHP2WSH => "p2sh-p2wsh",
            InputScriptType::P2WPKH => "p2wpkh",
            InputScriptType::P2WSH => "p2wsh",
            InputScriptType::P2TRKeyPath => "p2tr_keypath",
            InputScriptType::P2TRScriptPath => "p2tr_scriptpath",
            InputScriptType::Unknown => "unknown",
        }
    }
}

/// Classify an output scriptPubKey
pub fn classify_output(script: &[u8]) -> OutputScriptType {
    let n = script.len();
    match script {
        // P2PKH: OP_DUP OP_HASH160 OP_PUSHBYTES_20 <20B> OP_EQUALVERIFY OP_CHECKSIG
        s if n == 25
            && s[0] == 0x76
            && s[1] == 0xa9
            && s[2] == 0x14
            && s[23] == 0x88
            && s[24] == 0xac =>
        {
            OutputScriptType::P2PKH
        }
        // P2SH: OP_HASH160 OP_PUSHBYTES_20 <20B> OP_EQUAL
        s if n == 23 && s[0] == 0xa9 && s[1] == 0x14 && s[22] == 0x87 => OutputScriptType::P2SH,
        // P2WPKH: OP_0 OP_PUSHBYTES_20 <20B>
        s if n == 22 && s[0] == 0x00 && s[1] == 0x14 => OutputScriptType::P2WPKH,
        // P2WSH: OP_0 OP_PUSHBYTES_32 <32B>
        s if n == 34 && s[0] == 0x00 && s[1] == 0x20 => OutputScriptType::P2WSH,
        // P2TR: OP_1 OP_PUSHBYTES_32 <32B>
        s if n == 34 && s[0] == 0x51 && s[1] == 0x20 => OutputScriptType::P2TR,
        // OP_RETURN: starts with 0x6a
        s if !s.is_empty() && s[0] == 0x6a => OutputScriptType::OpReturn,
        _ => OutputScriptType::Unknown,
    }
}

/// Classify an input spend type based on:
/// - prevout scriptPubKey (base type)
/// - input scriptSig
/// - input witness stack
pub fn classify_input(
    prevout_script: &[u8],
    script_sig: &[u8],
    witness: &[Vec<u8>],
) -> InputScriptType {
    let prevout_type = classify_output(prevout_script);

    match prevout_type {
        OutputScriptType::P2PKH => InputScriptType::P2PKH,

        OutputScriptType::P2SH => {
            // Inspect the redeemScript (last push of scriptSig)
            if let Some(redeem_script) = last_push(script_sig) {
                let rs_len = redeem_script.len();
                // P2SH-P2WPKH: redeemScript = OP_0 <20B> (22 bytes)
                if rs_len == 22 && redeem_script[0] == 0x00 && redeem_script[1] == 0x14 {
                    return InputScriptType::P2SHP2WPKH;
                }
                // P2SH-P2WSH: redeemScript = OP_0 <32B> (34 bytes)
                if rs_len == 34 && redeem_script[0] == 0x00 && redeem_script[1] == 0x20 {
                    return InputScriptType::P2SHP2WSH;
                }
            }
            InputScriptType::Unknown
        }

        OutputScriptType::P2WPKH => InputScriptType::P2WPKH,

        OutputScriptType::P2WSH => InputScriptType::P2WSH,

        OutputScriptType::P2TR => classify_taproot_input(witness),

        _ => InputScriptType::Unknown,
    }
}

/// Classify a Taproot input as keypath or scriptpath
pub fn classify_taproot_input(witness: &[Vec<u8>]) -> InputScriptType {
    if witness.is_empty() {
        return InputScriptType::Unknown;
    }

    // Check for annex: if last item starts with 0x50, it's an annex
    let has_annex = witness
        .last()
        .map_or(false, |item| !item.is_empty() && item[0] == 0x50);

    let effective_witness: &[Vec<u8>] = if has_annex && witness.len() > 1 {
        &witness[..witness.len() - 1]
    } else {
        witness
    };

    match effective_witness.len() {
        // Keypath: exactly 1 item (64-byte Schnorr sig, or 65-byte with sighash type)
        1 => {
            let item = &effective_witness[0];
            if item.len() == 64 || item.len() == 65 {
                InputScriptType::P2TRKeyPath
            } else {
                InputScriptType::Unknown
            }
        }
        // Scriptpath: ≥2 items; last item before annex is the control block
        n if n >= 2 => {
            let control_block = effective_witness.last().unwrap();
            // Control block first byte: (leaf_version | parity) must have bits matching 0xc0/0xc1
            // specifically, leaf_version is in bits 7-1, bit 0 is parity
            // for Tapscript (OP_1 = 0xc0 + parity), first byte is 0xc0 or 0xc1
            if !control_block.is_empty() {
                let first = control_block[0];
                let leaf_version = first & 0xfe; // mask off parity bit
                                                 // Any valid leaf version is acceptable; length must be 33 + 32*n
                let len = control_block.len();
                if len >= 33 && (len - 33) % 32 == 0 {
                    return InputScriptType::P2TRScriptPath;
                }
            }
            InputScriptType::Unknown
        }
        _ => InputScriptType::Unknown,
    }
}

/// Extract the data from the last push opcode in a script.
/// Returns None if script is empty or last opcode is not a push.
pub fn last_push(script: &[u8]) -> Option<&[u8]> {
    if script.is_empty() {
        return None;
    }
    let mut pos = 0;
    let mut last: Option<&[u8]> = None;

    while pos < script.len() {
        let op = script[pos];
        pos += 1;
        match op {
            0x00 => {
                last = Some(&script[0..0]); // OP_0 pushes empty bytes
            }
            0x01..=0x4b => {
                let len = op as usize;
                if pos + len > script.len() {
                    return None;
                }
                last = Some(&script[pos..pos + len]);
                pos += len;
            }
            0x4c => {
                // OP_PUSHDATA1
                if pos >= script.len() {
                    return None;
                }
                let len = script[pos] as usize;
                pos += 1;
                if pos + len > script.len() {
                    return None;
                }
                last = Some(&script[pos..pos + len]);
                pos += len;
            }
            0x4d => {
                // OP_PUSHDATA2
                if pos + 2 > script.len() {
                    return None;
                }
                let len = u16::from_le_bytes([script[pos], script[pos + 1]]) as usize;
                pos += 2;
                if pos + len > script.len() {
                    return None;
                }
                last = Some(&script[pos..pos + len]);
                pos += len;
            }
            0x4e => {
                // OP_PUSHDATA4
                if pos + 4 > script.len() {
                    return None;
                }
                let len = u32::from_le_bytes([
                    script[pos],
                    script[pos + 1],
                    script[pos + 2],
                    script[pos + 3],
                ]) as usize;
                pos += 4;
                if pos + len > script.len() {
                    return None;
                }
                last = Some(&script[pos..pos + len]);
                pos += len;
            }
            _ => {
                last = None;
            }
        }
    }
    last
}
