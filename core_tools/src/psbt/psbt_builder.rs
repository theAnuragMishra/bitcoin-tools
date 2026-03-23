use crate::psbt::types::{BuilderError, Utxo};
use bitcoin::absolute::LockTime;
use bitcoin::base64::{engine::general_purpose::STANDARD, Engine};
use bitcoin::transaction::Version;
use bitcoin::{
    Amount, OutPoint, Psbt, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};

/// Build a PSBT from the selected inputs, outputs, and parameters.
pub fn build_psbt(
    selected_inputs: &[Utxo],
    payment_outputs: &[(ScriptBuf, u64)], // (script_pubkey, value_sats)
    change_output: Option<(ScriptBuf, u64)>, // (script_pubkey, value_sats)
    n_sequence: u32,
    n_locktime: u32,
) -> Result<Psbt, BuilderError> {
    // Build transaction inputs
    let mut tx_inputs: Vec<TxIn> = Vec::new();
    for utxo in selected_inputs {
        let txid: Txid = utxo.txid.parse().map_err(|e| {
            BuilderError::new("INVALID_TXID", &format!("Failed to parse txid: {}", e))
        })?;

        let outpoint = OutPoint::new(txid, utxo.vout);
        let txin = TxIn {
            previous_output: outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence(n_sequence),
            witness: Witness::default(),
        };
        tx_inputs.push(txin);
    }

    // Build transaction outputs
    let mut tx_outputs: Vec<TxOut> = Vec::new();
    for (script, value) in payment_outputs {
        tx_outputs.push(TxOut {
            value: Amount::from_sat(*value),
            script_pubkey: script.clone(),
        });
    }
    if let Some((script, value)) = &change_output {
        tx_outputs.push(TxOut {
            value: Amount::from_sat(*value),
            script_pubkey: script.clone(),
        });
    }

    let lock_time = LockTime::from_consensus(n_locktime);

    let unsigned_tx = Transaction {
        version: Version::TWO,
        lock_time,
        input: tx_inputs,
        output: tx_outputs,
    };

    let mut psbt = Psbt::from_unsigned_tx(unsigned_tx)
        .map_err(|e| BuilderError::new("PSBT_ERROR", &format!("Failed to create PSBT: {}", e)))?;

    // Set witness_utxo for each input
    for (i, utxo) in selected_inputs.iter().enumerate() {
        let script_pubkey = ScriptBuf::from_hex(&utxo.script_pubkey_hex).map_err(|e| {
            BuilderError::new(
                "INVALID_SCRIPT",
                &format!("Invalid script hex for input {}: {}", i, e),
            )
        })?;
        let witness_utxo = TxOut {
            value: Amount::from_sat(utxo.value_sats),
            script_pubkey,
        };
        psbt.inputs[i].witness_utxo = Some(witness_utxo);
    }

    Ok(psbt)
}

/// Serialize PSBT to base64
pub fn psbt_to_base64(psbt: &Psbt) -> String {
    let bytes = psbt.serialize();
    STANDARD.encode(&bytes)
}

/// Parse a hex script_pubkey string into a ScriptBuf
pub fn parse_script_pubkey(hex: &str) -> Result<ScriptBuf, BuilderError> {
    ScriptBuf::from_hex(hex).map_err(|e| {
        BuilderError::new(
            "INVALID_SCRIPT",
            &format!("Invalid script hex '{}': {}", hex, e),
        )
    })
}
