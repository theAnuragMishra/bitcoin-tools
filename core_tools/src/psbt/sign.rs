use std::collections::BTreeMap;

use bitcoin::base64::{engine::general_purpose::STANDARD, Engine};
use bitcoin::bip32::{DerivationPath, Fingerprint};
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::key::{PrivateKey, Secp256k1};
use bitcoin::psbt::PsbtSighashType;
use bitcoin::secp256k1::{Signing, Verification};
use bitcoin::{
    script, EcdsaSighashType, Psbt, PublicKey, ScriptBuf, TapSighashType, Witness, XOnlyPublicKey,
};

use crate::psbt::types::{BuilderError, ScriptType, Utxo};

pub fn try_sign_and_finalize_with_test_keys(
    psbt: &Psbt,
    selected_inputs: &[Utxo],
    maybe_wifs: Option<&[String]>,
) -> Result<(Option<String>, Option<String>), BuilderError> {
    let Some(wifs) = maybe_wifs else {
        return Ok((None, None));
    };
    if wifs.is_empty() {
        return Ok((None, None));
    }

    let secp = Secp256k1::new();
    let mut ecdsa_keys: BTreeMap<PublicKey, PrivateKey> = BTreeMap::new();
    let mut xonly_keys: BTreeMap<XOnlyPublicKey, PrivateKey> = BTreeMap::new();

    for wif in wifs {
        let sk = PrivateKey::from_wif(wif).map_err(|e| {
            BuilderError::new(
                "INVALID_FIXTURE",
                &format!("Invalid test_signing_wifs entry: {}", e),
            )
        })?;
        let pubkey = sk.public_key(&secp);
        ecdsa_keys.insert(pubkey, sk);

        let (xonly, _) = sk.inner.x_only_public_key(&secp);
        xonly_keys.insert(xonly, sk);
    }

    let mut signed = psbt.clone();
    annotate_inputs_for_signing(
        &mut signed,
        selected_inputs,
        &ecdsa_keys,
        &xonly_keys,
        &secp,
    )?;

    let _ = signed.sign(&ecdsa_keys, &secp);
    let _ = signed.sign(&xonly_keys, &secp);

    finalize_supported_inputs(&mut signed, selected_inputs)?;

    let all_finalized = signed
        .inputs
        .iter()
        .all(|i| i.final_script_sig.is_some() || i.final_script_witness.is_some());
    if !all_finalized {
        return Ok((Some(STANDARD.encode(signed.serialize())), None));
    }

    let signed_psbt_base64 = STANDARD.encode(signed.serialize());
    let tx = signed.extract_tx_unchecked_fee_rate();
    let tx_hex = serialize_hex(&tx);

    Ok((Some(signed_psbt_base64), Some(tx_hex)))
}

fn annotate_inputs_for_signing<C: Signing + Verification>(
    psbt: &mut Psbt,
    selected_inputs: &[Utxo],
    ecdsa_keys: &BTreeMap<PublicKey, PrivateKey>,
    xonly_keys: &BTreeMap<XOnlyPublicKey, PrivateKey>,
    secp: &Secp256k1<C>,
) -> Result<(), BuilderError> {
    for (idx, utxo) in selected_inputs.iter().enumerate() {
        let script = ScriptBuf::from_hex(&utxo.script_pubkey_hex).map_err(|e| {
            BuilderError::new(
                "INVALID_FIXTURE",
                &format!("Invalid selected input script at {}: {}", idx, e),
            )
        })?;

        match utxo.script_type {
            ScriptType::P2wpkh | ScriptType::P2shP2wpkh | ScriptType::P2pkh => {
                for (pk, _sk) in ecdsa_keys {
                    let Ok(wpkh) = pk.wpubkey_hash() else {
                        continue;
                    };
                    let p2wpkh_script = ScriptBuf::new_p2wpkh(&wpkh);
                    let p2sh_p2wpkh_script = p2wpkh_script.to_p2sh();
                    let p2pkh_script = ScriptBuf::new_p2pkh(&pk.pubkey_hash());

                    let matches = match utxo.script_type {
                        ScriptType::P2wpkh => script == p2wpkh_script,
                        ScriptType::P2shP2wpkh => script == p2sh_p2wpkh_script,
                        ScriptType::P2pkh => script == p2pkh_script,
                        _ => false,
                    };
                    if !matches {
                        continue;
                    }

                    psbt.inputs[idx].bip32_derivation.insert(
                        pk.inner,
                        (Fingerprint::default(), DerivationPath::default()),
                    );
                    psbt.inputs[idx].sighash_type =
                        Some(PsbtSighashType::from(EcdsaSighashType::All));
                    if utxo.script_type == ScriptType::P2shP2wpkh {
                        psbt.inputs[idx].redeem_script = Some(p2wpkh_script);
                    }
                    break;
                }
            }
            ScriptType::P2tr => {
                for (xonly, _sk) in xonly_keys {
                    let tr_script = ScriptBuf::new_p2tr(secp, *xonly, None);
                    if tr_script != script {
                        continue;
                    }

                    psbt.inputs[idx].tap_internal_key = Some(*xonly);
                    psbt.inputs[idx].tap_key_origins.insert(
                        *xonly,
                        (vec![], (Fingerprint::default(), DerivationPath::default())),
                    );
                    psbt.inputs[idx].sighash_type =
                        Some(PsbtSighashType::from(TapSighashType::Default));
                    break;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn finalize_supported_inputs(
    psbt: &mut Psbt,
    selected_inputs: &[Utxo],
) -> Result<(), BuilderError> {
    for (idx, utxo) in selected_inputs.iter().enumerate() {
        match utxo.script_type {
            ScriptType::P2wpkh => {
                let Some((pk, sig)) = psbt.inputs[idx].partial_sigs.iter().next() else {
                    continue;
                };
                let witness = Witness::p2wpkh(sig, &pk.inner);
                psbt.inputs[idx].final_script_witness = Some(witness);
            }
            ScriptType::P2shP2wpkh => {
                let Some((pk, sig)) = psbt.inputs[idx].partial_sigs.iter().next() else {
                    continue;
                };
                let witness = Witness::p2wpkh(sig, &pk.inner);
                psbt.inputs[idx].final_script_witness = Some(witness);

                let redeem = psbt.inputs[idx].redeem_script.clone().ok_or_else(|| {
                    BuilderError::new(
                        "PSBT_ERROR",
                        "Missing redeem script for p2sh-p2wpkh finalization",
                    )
                })?;
                let redeem_bytes =
                    script::PushBytesBuf::try_from(redeem.to_bytes()).map_err(|e| {
                        BuilderError::new(
                            "PSBT_ERROR",
                            &format!("Redeem script push failed: {}", e),
                        )
                    })?;
                psbt.inputs[idx].final_script_sig = Some(
                    script::Builder::new()
                        .push_slice(redeem_bytes)
                        .into_script(),
                );
            }
            ScriptType::P2pkh => {
                let Some((pk, sig)) = psbt.inputs[idx].partial_sigs.iter().next() else {
                    continue;
                };

                let sig_bytes =
                    script::PushBytesBuf::try_from(sig.serialize().to_vec()).map_err(|e| {
                        BuilderError::new(
                            "PSBT_ERROR",
                            &format!("P2PKH signature push failed: {}", e),
                        )
                    })?;
                let pubkey_bytes = script::PushBytesBuf::try_from(pk.to_bytes()).map_err(|e| {
                    BuilderError::new("PSBT_ERROR", &format!("P2PKH pubkey push failed: {}", e))
                })?;

                psbt.inputs[idx].final_script_sig = Some(
                    script::Builder::new()
                        .push_slice(sig_bytes)
                        .push_slice(pubkey_bytes)
                        .into_script(),
                );
            }
            ScriptType::P2tr => {
                let Some(sig) = psbt.inputs[idx].tap_key_sig else {
                    continue;
                };
                psbt.inputs[idx].final_script_witness = Some(Witness::p2tr_key_spend(&sig));
            }
            _ => {}
        }

        psbt.inputs[idx].partial_sigs = BTreeMap::new();
        psbt.inputs[idx].sighash_type = None;
        psbt.inputs[idx].bip32_derivation = BTreeMap::new();
        psbt.inputs[idx].tap_key_origins = BTreeMap::new();
    }

    Ok(())
}
