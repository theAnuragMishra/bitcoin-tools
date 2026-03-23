import { useState } from "react";
import type { Vin } from "../types";
import Tooltip from "./Tooltip";
import ScriptTypeBadge from "./ScriptTypeBadge";
import { sats } from "../utils/format";

export default function VinRow({ inp, index }: { inp: Vin; index: number }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div
      style={{
        marginBottom: 12,
        background: "var(--surface2)",
        borderRadius: "var(--radius)",
        border: "1px solid var(--border)",
        padding: "10px 12px",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          gap: 8,
        }}
      >
        <div>
          <span style={{ color: "var(--text2)", fontSize: 12 }}>
            Input #{index} ·{" "}
          </span>
          <strong style={{ color: "var(--accent)" }}>
            {sats(inp.prevout.value_sats)}
          </strong>
          <span style={{ marginLeft: 8 }}>
            <ScriptTypeBadge scriptType={inp.script_type} />
          </span>
          {inp.address && (
            <div className="hex" style={{ marginTop: 4 }}>
              From: {inp.address}
            </div>
          )}
        </div>
        <button className="detail-btn" onClick={() => setExpanded(!expanded)}>
          {expanded ? "▲ less" : "▼ more"}
        </button>
      </div>
      {inp.relative_timelock.enabled && (
        <div style={{ marginTop: 6, fontSize: 12, color: "var(--yellow)" }}>
          ⏱ Relative timelock: {inp.relative_timelock.value}{" "}
          {inp.relative_timelock.type}
          <span style={{ color: "var(--text2)", marginLeft: 6 }}>
            (sequence: 0x{inp.sequence.toString(16).toUpperCase()})
          </span>
        </div>
      )}
      {expanded && (
        <div style={{ marginTop: 10, fontSize: 12 }}>
          <div style={{ marginBottom: 6 }}>
            <span style={{ color: "var(--text2)" }}>Spends: </span>
            <span className="hex">
              {inp.txid}:{inp.vout}
            </span>
          </div>
          <div style={{ marginBottom: 6 }}>
            <span style={{ color: "var(--text2)" }}>
              <Tooltip tip="Sequence enables relative timelocks and RBF signaling.">
                Sequence
              </Tooltip>
              :{" "}
            </span>
            <span className="hex">
              0x{inp.sequence.toString(16).toUpperCase()}
            </span>
          </div>
          {inp.script_sig_hex && (
            <div style={{ marginBottom: 6 }}>
              <span style={{ color: "var(--text2)" }}>
                <Tooltip tip="Unlocking script provided by the spender.">
                  scriptSig (ASM)
                </Tooltip>
                :{" "}
              </span>
              <span className="hex">{inp.script_asm || "(empty)"}</span>
            </div>
          )}
          {inp.witness.length > 0 && (
            <div style={{ marginBottom: 6 }}>
              <span style={{ color: "var(--text2)" }}>
                <Tooltip tip="SegWit witness data containing signatures.">
                  Witness
                </Tooltip>{" "}
                ({inp.witness.length} items):
              </span>
              {inp.witness.map((w, j) => (
                <div key={j} className="hex" style={{ marginLeft: 12 }}>
                  [{j}] {w || "(empty)"}
                </div>
              ))}
            </div>
          )}
          {inp.witness_script_asm && (
            <div>
              <span style={{ color: "var(--text2)" }}>
                <Tooltip tip="Script revealed for SegWit spends.">
                  Witness script (ASM)
                </Tooltip>
                :{" "}
              </span>
              <span className="hex">{inp.witness_script_asm}</span>
            </div>
          )}
          <div style={{ marginTop: 6 }}>
            <span style={{ color: "var(--text2)" }}>
              <Tooltip tip="Locking script of the previous output being spent.">
                Previous output script
              </Tooltip>
              :{" "}
            </span>
            <span className="hex">{inp.prevout.script_pubkey_hex}</span>
          </div>
        </div>
      )}
    </div>
  );
}
