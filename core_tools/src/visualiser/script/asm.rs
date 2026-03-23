/// Disassemble a Bitcoin script to ASM string format.
/// Empty script returns "".
pub fn disassemble(script: &[u8]) -> String {
    if script.is_empty() {
        return String::new();
    }

    let mut tokens = Vec::new();
    let mut pos = 0;

    while pos < script.len() {
        let op = script[pos];
        pos += 1;

        let token = match op {
            // Special push: OP_0
            0x00 => "OP_0".to_string(),

            // Direct pushes OP_PUSHBYTES_1 .. OP_PUSHBYTES_75
            0x01..=0x4b => {
                let len = op as usize;
                if pos + len > script.len() {
                    tokens.push(format!("OP_PUSHBYTES_{} [truncated]", len));
                    break;
                }
                let data = hex::encode(&script[pos..pos + len]);
                pos += len;
                format!("OP_PUSHBYTES_{} {}", len, data)
            }

            // OP_PUSHDATA1
            0x4c => {
                if pos >= script.len() {
                    tokens.push("OP_PUSHDATA1 [truncated]".to_string());
                    break;
                }
                let len = script[pos] as usize;
                pos += 1;
                if pos + len > script.len() {
                    tokens.push(format!("OP_PUSHDATA1 [truncated]"));
                    break;
                }
                let data = hex::encode(&script[pos..pos + len]);
                pos += len;
                format!("OP_PUSHDATA1 {}", data)
            }

            // OP_PUSHDATA2
            0x4d => {
                if pos + 2 > script.len() {
                    tokens.push("OP_PUSHDATA2 [truncated]".to_string());
                    break;
                }
                let len = u16::from_le_bytes([script[pos], script[pos + 1]]) as usize;
                pos += 2;
                if pos + len > script.len() {
                    tokens.push("OP_PUSHDATA2 [truncated]".to_string());
                    break;
                }
                let data = hex::encode(&script[pos..pos + len]);
                pos += len;
                format!("OP_PUSHDATA2 {}", data)
            }

            // OP_PUSHDATA4
            0x4e => {
                if pos + 4 > script.len() {
                    tokens.push("OP_PUSHDATA4 [truncated]".to_string());
                    break;
                }
                let len = u32::from_le_bytes([
                    script[pos],
                    script[pos + 1],
                    script[pos + 2],
                    script[pos + 3],
                ]) as usize;
                pos += 4;
                if pos + len > script.len() {
                    tokens.push("OP_PUSHDATA4 [truncated]".to_string());
                    break;
                }
                let data = hex::encode(&script[pos..pos + len]);
                pos += len;
                format!("OP_PUSHDATA4 {}", data)
            }

            // OP_1NEGATE
            0x4f => "OP_1NEGATE".to_string(),
            // OP_RESERVED
            0x50 => "OP_RESERVED".to_string(),
            // OP_1 .. OP_16
            0x51 => "OP_1".to_string(),
            0x52 => "OP_2".to_string(),
            0x53 => "OP_3".to_string(),
            0x54 => "OP_4".to_string(),
            0x55 => "OP_5".to_string(),
            0x56 => "OP_6".to_string(),
            0x57 => "OP_7".to_string(),
            0x58 => "OP_8".to_string(),
            0x59 => "OP_9".to_string(),
            0x5a => "OP_10".to_string(),
            0x5b => "OP_11".to_string(),
            0x5c => "OP_12".to_string(),
            0x5d => "OP_13".to_string(),
            0x5e => "OP_14".to_string(),
            0x5f => "OP_15".to_string(),
            0x60 => "OP_16".to_string(),

            // Flow control
            0x61 => "OP_NOP".to_string(),
            0x62 => "OP_VER".to_string(),
            0x63 => "OP_IF".to_string(),
            0x64 => "OP_NOTIF".to_string(),
            0x65 => "OP_VERIF".to_string(),
            0x66 => "OP_VERNOTIF".to_string(),
            0x67 => "OP_ELSE".to_string(),
            0x68 => "OP_ENDIF".to_string(),
            0x69 => "OP_VERIFY".to_string(),
            0x6a => "OP_RETURN".to_string(),

            // Stack ops
            0x6b => "OP_TOALTSTACK".to_string(),
            0x6c => "OP_FROMALTSTACK".to_string(),
            0x6d => "OP_2DROP".to_string(),
            0x6e => "OP_2DUP".to_string(),
            0x6f => "OP_3DUP".to_string(),
            0x70 => "OP_2OVER".to_string(),
            0x71 => "OP_2ROT".to_string(),
            0x72 => "OP_2SWAP".to_string(),
            0x73 => "OP_IFDUP".to_string(),
            0x74 => "OP_DEPTH".to_string(),
            0x75 => "OP_DROP".to_string(),
            0x76 => "OP_DUP".to_string(),
            0x77 => "OP_NIP".to_string(),
            0x78 => "OP_OVER".to_string(),
            0x79 => "OP_PICK".to_string(),
            0x7a => "OP_ROLL".to_string(),
            0x7b => "OP_ROT".to_string(),
            0x7c => "OP_SWAP".to_string(),
            0x7d => "OP_TUCK".to_string(),

            // Splice ops
            0x7e => "OP_CAT".to_string(),
            0x7f => "OP_SUBSTR".to_string(),
            0x80 => "OP_LEFT".to_string(),
            0x81 => "OP_RIGHT".to_string(),
            0x82 => "OP_SIZE".to_string(),

            // Bit logic
            0x83 => "OP_INVERT".to_string(),
            0x84 => "OP_AND".to_string(),
            0x85 => "OP_OR".to_string(),
            0x86 => "OP_XOR".to_string(),
            0x87 => "OP_EQUAL".to_string(),
            0x88 => "OP_EQUALVERIFY".to_string(),
            0x89 => "OP_RESERVED1".to_string(),
            0x8a => "OP_RESERVED2".to_string(),

            // Numeric
            0x8b => "OP_1ADD".to_string(),
            0x8c => "OP_1SUB".to_string(),
            0x8d => "OP_2MUL".to_string(),
            0x8e => "OP_2DIV".to_string(),
            0x8f => "OP_NEGATE".to_string(),
            0x90 => "OP_ABS".to_string(),
            0x91 => "OP_NOT".to_string(),
            0x92 => "OP_0NOTEQUAL".to_string(),
            0x93 => "OP_ADD".to_string(),
            0x94 => "OP_SUB".to_string(),
            0x95 => "OP_MUL".to_string(),
            0x96 => "OP_DIV".to_string(),
            0x97 => "OP_MOD".to_string(),
            0x98 => "OP_LSHIFT".to_string(),
            0x99 => "OP_RSHIFT".to_string(),
            0x9a => "OP_BOOLAND".to_string(),
            0x9b => "OP_BOOLOR".to_string(),
            0x9c => "OP_NUMEQUAL".to_string(),
            0x9d => "OP_NUMEQUALVERIFY".to_string(),
            0x9e => "OP_NUMNOTEQUAL".to_string(),
            0x9f => "OP_LESSTHAN".to_string(),
            0xa0 => "OP_GREATERTHAN".to_string(),
            0xa1 => "OP_LESSTHANOREQUAL".to_string(),
            0xa2 => "OP_GREATERTHANOREQUAL".to_string(),
            0xa3 => "OP_MIN".to_string(),
            0xa4 => "OP_MAX".to_string(),
            0xa5 => "OP_WITHIN".to_string(),

            // Crypto
            0xa6 => "OP_RIPEMD160".to_string(),
            0xa7 => "OP_SHA1".to_string(),
            0xa8 => "OP_SHA256".to_string(),
            0xa9 => "OP_HASH160".to_string(),
            0xaa => "OP_HASH256".to_string(),
            0xab => "OP_CODESEPARATOR".to_string(),
            0xac => "OP_CHECKSIG".to_string(),
            0xad => "OP_CHECKSIGVERIFY".to_string(),
            0xae => "OP_CHECKMULTISIG".to_string(),
            0xaf => "OP_CHECKMULTISIGVERIFY".to_string(),

            // NOP expansions
            0xb0 => "OP_NOP1".to_string(),
            0xb1 => "OP_CHECKLOCKTIMEVERIFY".to_string(),
            0xb2 => "OP_CHECKSEQUENCEVERIFY".to_string(),
            0xb3 => "OP_NOP4".to_string(),
            0xb4 => "OP_NOP5".to_string(),
            0xb5 => "OP_NOP6".to_string(),
            0xb6 => "OP_NOP7".to_string(),
            0xb7 => "OP_NOP8".to_string(),
            0xb8 => "OP_NOP9".to_string(),
            0xb9 => "OP_NOP10".to_string(),

            // Taproot/Tapscript
            0xba => "OP_CHECKSIGADD".to_string(),

            // Anything else
            other => format!("OP_UNKNOWN_{:#04x}", other),
        };

        tokens.push(token);
    }

    tokens.join(" ")
}

/// Collect OP_RETURN payload data (concatenation of all push operations after OP_RETURN).
/// Returns the raw bytes.
pub fn collect_op_return_data(script: &[u8]) -> Vec<u8> {
    if script.is_empty() || script[0] != 0x6a {
        return Vec::new();
    }

    let mut data = Vec::new();
    let mut pos = 1; // skip OP_RETURN

    while pos < script.len() {
        let op = script[pos];
        pos += 1;
        match op {
            0x00 => {
                // OP_0: push empty bytes
                // (no data to add)
            }
            0x01..=0x4b => {
                let len = op as usize;
                if pos + len > script.len() {
                    break;
                }
                data.extend_from_slice(&script[pos..pos + len]);
                pos += len;
            }
            0x4c => {
                if pos >= script.len() {
                    break;
                }
                let len = script[pos] as usize;
                pos += 1;
                if pos + len > script.len() {
                    break;
                }
                data.extend_from_slice(&script[pos..pos + len]);
                pos += len;
            }
            0x4d => {
                if pos + 2 > script.len() {
                    break;
                }
                let len = u16::from_le_bytes([script[pos], script[pos + 1]]) as usize;
                pos += 2;
                if pos + len > script.len() {
                    break;
                }
                data.extend_from_slice(&script[pos..pos + len]);
                pos += len;
            }
            0x4e => {
                if pos + 4 > script.len() {
                    break;
                }
                let len = u32::from_le_bytes([
                    script[pos],
                    script[pos + 1],
                    script[pos + 2],
                    script[pos + 3],
                ]) as usize;
                pos += 4;
                if pos + len > script.len() {
                    break;
                }
                data.extend_from_slice(&script[pos..pos + len]);
                pos += len;
            }
            _ => {
                // Non-push opcode; stop collection
                break;
            }
        }
    }

    data
}

/// Detect OP_RETURN protocol from raw data bytes
pub fn detect_op_return_protocol(data: &[u8]) -> &'static str {
    if data.starts_with(&[0x6f, 0x6d, 0x6e, 0x69]) {
        "omni"
    } else if data.starts_with(&[0x01, 0x09, 0xf9, 0x11, 0x02]) {
        "opentimestamps"
    } else {
        "unknown"
    }
}
