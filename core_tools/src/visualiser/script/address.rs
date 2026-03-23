use bech32::{hrp, segwit};

/// Derive address from a scriptPubKey for mainnet.
/// Returns None for unknown/op_return types.
pub fn script_to_address(script: &[u8]) -> Option<String> {
    let n = script.len();

    // P2PKH: OP_DUP OP_HASH160 OP_PUSHBYTES_20 <20B> OP_EQUALVERIFY OP_CHECKSIG
    if n == 25
        && script[0] == 0x76
        && script[1] == 0xa9
        && script[2] == 0x14
        && script[23] == 0x88
        && script[24] == 0xac
    {
        let hash = &script[3..23];
        return Some(p2pkh_address(hash));
    }

    // P2SH: OP_HASH160 OP_PUSHBYTES_20 <20B> OP_EQUAL
    if n == 23 && script[0] == 0xa9 && script[1] == 0x14 && script[22] == 0x87 {
        let hash = &script[2..22];
        return Some(p2sh_address(hash));
    }

    // P2WPKH: OP_0 OP_PUSHBYTES_20 <20B>
    if n == 22 && script[0] == 0x00 && script[1] == 0x14 {
        let program = &script[2..22];
        return segwit::encode_v0(hrp::BC, program).ok();
    }

    // P2WSH: OP_0 OP_PUSHBYTES_32 <32B>
    if n == 34 && script[0] == 0x00 && script[1] == 0x20 {
        let program = &script[2..34];
        return segwit::encode_v0(hrp::BC, program).ok();
    }

    // P2TR: OP_1 OP_PUSHBYTES_32 <32B>
    if n == 34 && script[0] == 0x51 && script[1] == 0x20 {
        let program = &script[2..34];
        return segwit::encode_v1(hrp::BC, program).ok();
    }

    None
}

/// Mainnet P2PKH: base58check with version byte 0x00
pub fn p2pkh_address(hash20: &[u8]) -> String {
    let mut payload = Vec::with_capacity(21);
    payload.push(0x00u8);
    payload.extend_from_slice(hash20);
    bs58::encode(payload).with_check().into_string()
}

/// Mainnet P2SH: base58check with version byte 0x05
pub fn p2sh_address(hash20: &[u8]) -> String {
    let mut payload = Vec::with_capacity(21);
    payload.push(0x05u8);
    payload.extend_from_slice(hash20);
    bs58::encode(payload).with_check().into_string()
}
