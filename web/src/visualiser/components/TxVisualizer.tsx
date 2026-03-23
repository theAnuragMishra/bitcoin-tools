import { useState } from "react";
import type { TxReport } from "../types";
import Tooltip from "./Tooltip";
import ScriptTypeBadge from "./ScriptTypeBadge";
import Section from "./Section";
import SegwitPie from "./SegwitPie";
import VinRow from "./VinRow";
import { sats, shortHex } from "../utils/format";

export default function TxVisualizer({ report }: { report: TxReport }) {
  const [showTech, setShowTech] = useState(false);

  const totalIn = report.total_input_sats;
  const totalOut = report.total_output_sats;
  const fee = report.fee_sats;
  const segwit = report.segwit_savings;
  const actualWeight = segwit?.weight_actual ?? 0;
  const legacyWeight = segwit?.weight_if_legacy ?? 0;
  const weightMax = Math.max(actualWeight, legacyWeight, 1);
  const chartHeight = 140;
  const chartBase = 170;
  const actualBarHeight = Math.round((actualWeight / weightMax) * chartHeight);
  const legacyBarHeight = Math.round((legacyWeight / weightMax) * chartHeight);

  return (
    <div className="report">
      {report.warnings.length > 0 && (
        <div className="warnings">
          {report.warnings.map((w, i) => {
            const cls =
              w.code === "RBF_SIGNALING"
                ? "w-rbf"
                : w.code === "HIGH_FEE"
                  ? "w-high"
                  : w.code === "DUST_OUTPUT"
                    ? "w-dust"
                    : "w-unknown";
            const label =
              {
                RBF_SIGNALING: "⚡ Replaceable (RBF)",
                HIGH_FEE: "🔴 High Fee",
                DUST_OUTPUT: "🟣 Dust Output",
                UNKNOWN_OUTPUT_SCRIPT: "❓ Unknown Script",
              }[w.code] || w.code;
            const tip =
              {
                RBF_SIGNALING:
                  'This transaction can be replaced with a higher-fee version before it confirms. This is called "Replace-By-Fee" (RBF).',
                HIGH_FEE: "This transaction is paying an unusually high fee.",
                DUST_OUTPUT:
                  "One or more outputs have very tiny amounts (less than 546 satoshis) that may not be economically spendable.",
                UNKNOWN_OUTPUT_SCRIPT:
                  "One or more outputs have an unrecognized script pattern.",
              }[w.code] || "";
            return (
              <Tooltip key={i} tip={tip}>
                <span className={`w-chip ${cls}`}>{label}</span>
              </Tooltip>
            );
          })}
        </div>
      )}

      <div className="story-cards">
        <div className="story-card">
          <div className="s-label">
            <Tooltip tip="A unique identifier for this transaction.">
              Transaction ID
            </Tooltip>
          </div>
          <div
            className="s-value"
            style={{
              fontSize: 12,
              fontFamily: "var(--mono)",
              wordBreak: "break-all",
            }}
          >
            {report.txid}
          </div>
          <div className="s-sub">
            {report.segwit ? (
              <span className="badge badge-sw">SegWit</span>
            ) : (
              <span className="badge badge-legacy">Legacy</span>
            )}
          </div>
        </div>
        <div className="story-card">
          <div className="s-label">
            <Tooltip tip="The fee is what inputs minus outputs equals — the miner's reward for including this transaction in a block.">
              Fee paid
            </Tooltip>
          </div>
          <div className="s-value v-accent">{sats(fee)}</div>
          <div className="s-sub">{report.fee_rate_sat_vb} sat/vB</div>
        </div>
        <div className="story-card">
          <div className="s-label">
            <Tooltip tip="Virtual bytes (vbytes) is a weighted measure of transaction size. SegWit witness data counts as 1/4 of a non-witness byte.">
              Size (vbytes)
            </Tooltip>
          </div>
          <div className="s-value v-blue">{report.vbytes}</div>
          <div className="s-sub">{report.size_bytes} bytes total</div>
        </div>
        <div className="story-card">
          <div className="s-label">
            <Tooltip tip="Inputs spend old coins, outputs create new coins for recipients.">
              Inputs / Outputs
            </Tooltip>
          </div>
          <div className="s-value">
            {report.vin.length} / {report.vout.length}
          </div>
          <div className="s-sub">
            {sats(totalIn)} in → {sats(totalOut)} out
          </div>
        </div>
        <div className="story-card">
          <div className="s-label">
            <Tooltip tip="Locktime prevents the transaction from being mined before a certain block height or timestamp.">
              Locktime
            </Tooltip>
          </div>
          <div className="s-value" style={{ fontSize: 16 }}>
            {report.locktime_type === "none"
              ? "None"
              : report.locktime_value.toLocaleString()}
          </div>
          <div className="s-sub">{report.locktime_type}</div>
        </div>
        <div className="story-card">
          <div className="s-label">
            <Tooltip tip="Weight is calculated as: (non-witness bytes × 4) + witness bytes. The limit per block is 4,000,000 weight units.">
              Weight
            </Tooltip>
          </div>
          <div className="s-value">{report.weight.toLocaleString()}</div>
          <div className="s-sub">WU (max 4,000,000 per block)</div>
        </div>
      </div>

      <Section title="💸 Value Flow — Who pays whom?">
        <div className="flow-diagram">
          <div className="flow-col">
            <div className="flow-col-label">Inputs (being spent)</div>
            {report.vin.map((inp, i) => (
              <div className="flow-box" key={i}>
                <div className="f-amount">{sats(inp.prevout.value_sats)}</div>
                <div className="f-type">
                  <ScriptTypeBadge scriptType={inp.script_type} />
                </div>
                {inp.address && (
                  <div className="f-addr">
                    <span className="f-addr-label">Address: </span>
                    {shortHex(inp.address, 12)}
                  </div>
                )}
              </div>
            ))}
          </div>

          <div className="flow-middle">
            <div className="flow-arrow-line"></div>
            <div className="flow-center-box">
              <div className="flow-center-icon">₿</div>
              <div className="flow-center-label">Bitcoin Transaction</div>

              <Tooltip
                tip="The fee is the difference between total inputs and total
                outputs. It is collected by the miner who includes this
                transaction in a block."
              >
                <div className="flow-fee-chip">
                  <span
                    style={{ borderBottom: "1px dashed var(--text2)" }}
                    className="tt-wrap"
                  >
                    Fee: {sats(fee)}
                  </span>
                  <div className="fee-rate">
                    {report.fee_rate_sat_vb} sat/vB
                  </div>
                </div>
              </Tooltip>
            </div>
            <div className="flow-arrow-line"></div>
          </div>
          <div className="flow-col">
            <div className="flow-col-label">Outputs (new recipients)</div>
            {report.vout.map((out, i) => (
              <div className="flow-box" key={i}>
                {out.script_type === "op_return" ? (
                  <>
                    <div className="f-amount">OP_RETURN</div>
                    <div className="f-type">
                      <ScriptTypeBadge scriptType={out.script_type} />
                    </div>
                    {out.op_return_data_utf8 && (
                      <div className="f-addr">
                        <span className="f-addr-label">Data</span>
                        {String(out.op_return_data_utf8).slice(0, 40)}
                      </div>
                    )}
                  </>
                ) : (
                  <>
                    <div className="f-amount">{sats(out.value_sats)}</div>
                    <div className="f-type">
                      <ScriptTypeBadge scriptType={out.script_type} />
                    </div>
                    {out.address && (
                      <div className="f-addr">
                        <span className="f-addr-label">Address: </span>
                        {shortHex(out.address, 12)}
                      </div>
                    )}
                  </>
                )}
              </div>
            ))}
          </div>
        </div>
      </Section>

      {segwit && (
        <Section title="⚡ SegWit Discount — Why witness data is cheaper">
          <p className="text-sm" style={{ marginBottom: 12 }}>
            <strong>SegWit (Segregated Witness)</strong> moved signature data
            into a separate "witness" area. Witness bytes are counted at 1/4
            weight — so the same transaction costs less to include in a block
            compared to legacy format.
          </p>
          <div className="segwit-wrap">
            <div className="segwit-card">
              <div className="segwit-stats-grid">
                <div className="segwit-stat">
                  <span
                    style={{ background: "rgb(247, 147, 26)" }}
                    className="segwit-dot"
                  ></span>
                  <div>
                    <div className="stat-label">None witness (base) data</div>
                    <div
                      style={{ fontWeight: 700, color: "rgb(247, 147, 26)" }}
                    >
                      {segwit.non_witness_bytes.toLocaleString()} bytes
                    </div>
                    <div className="text-sm">Counts as 4 weight each</div>
                  </div>
                </div>
                <div className="segwit-stat">
                  <span
                    className="segwit-dot"
                    style={{ background: "rgb(88, 166, 255)" }}
                  ></span>
                  <div>
                    <div className="stat-label">Witness (signature) data</div>
                    <div
                      style={{ fontWeight: 700, color: "rgb(88, 166, 255)" }}
                    >
                      {segwit.witness_bytes.toLocaleString()} bytes
                    </div>
                    <div className="text-sm">
                      Counts as 1 weight each (75% discount)
                    </div>
                  </div>
                </div>
                <div className="segwit-stat">
                  <span
                    className="segwit-dot"
                    style={{ background: "rgb(63, 185, 80)" }}
                  ></span>
                  <div>
                    <div className="stat-label">Weight saved vs legacy</div>
                    <div style={{ fontWeight: 700, color: "rgb(63, 185, 80)" }}>
                      {segwit.savings_pct}% saved
                    </div>
                    <div className="text-sm">
                      {segwit.non_witness_bytes * 4 + segwit.witness_bytes} WU
                      actual vs {segwit.total_bytes * 4} WU legacy
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div className="segwit-bars">
              <div className="s-label" style={{ marginBottom: 2 }}>
                Weight comparison
              </div>
              <svg
                className="weight-chart"
                viewBox="0 0 240 200"
                role="img"
                aria-label="Bar chart comparing actual SegWit weight to hypothetical legacy weight"
              >
                <line
                  x1="30"
                  y1="20"
                  x2="30"
                  y2="170"
                  className="weight-axis"
                />
                <line
                  x1="30"
                  y1="170"
                  x2="220"
                  y2="170"
                  className="weight-axis"
                />
                <rect
                  x="70"
                  y={chartBase - actualBarHeight}
                  width="44"
                  height={actualBarHeight}
                  rx="6"
                  className="weight-bar weight-bar-actual"
                />
                <rect
                  x="140"
                  y={chartBase - legacyBarHeight}
                  width="44"
                  height={legacyBarHeight}
                  rx="6"
                  className="weight-bar weight-bar-legacy"
                />
                <text
                  x="92"
                  y="190"
                  textAnchor="middle"
                  className="weight-label"
                >
                  Actual
                </text>
                <text
                  x="162"
                  y="190"
                  textAnchor="middle"
                  className="weight-label"
                >
                  Legacy
                </text>
                <text
                  x="92"
                  y={chartBase - actualBarHeight - 8}
                  textAnchor="middle"
                  className="weight-value"
                >
                  {actualWeight.toLocaleString()} WU
                </text>
                <text
                  x="162"
                  y={chartBase - legacyBarHeight - 8}
                  textAnchor="middle"
                  className="weight-value"
                >
                  {legacyWeight.toLocaleString()} WU
                </text>
              </svg>
              <div className="text-sm" style={{ textAlign: "center" }}>
                Taller bar = more weight to include in a block
              </div>
            </div>
            <div className="segwit-visual">
              <div className="s-label" style={{ marginBottom: 8 }}>
                Witness Weight Contributions
              </div>
              <div style={{ display: "inline-flex", flexDirection: "column" }}>
                <SegwitPie
                  witness={segwit.witness_bytes}
                  nonWitness={segwit.non_witness_bytes}
                  segwit={true}
                />
                <p>Segwit</p>
              </div>
              <div style={{ display: "inline-flex", flexDirection: "column" }}>
                <SegwitPie
                  witness={segwit.witness_bytes}
                  nonWitness={segwit.non_witness_bytes}
                  segwit={false}
                />
                <p>Legacy</p>
              </div>
            </div>
          </div>
        </Section>
      )}

      <Section title={`📥 Inputs (${report.vin.length}) — What is being spent`}>
        <p className="text-sm" style={{ marginBottom: 12 }}>
          Each input references a previous output (UTXO) that is being
          "unlocked" and spent. The input includes a scriptSig (and/or witness
          data) that proves ownership.
        </p>
        {report.vin.map((inp, i) => (
          <VinRow key={i} inp={inp} index={i} />
        ))}
      </Section>

      <Section
        title={`📤 Outputs (${report.vout.length}) — What is being created`}
      >
        <p className="text-sm" style={{ marginBottom: 12 }}>
          Each output creates a new UTXO (unspent transaction output) locked to
          a recipient's address. It can be spent in a future transaction.
        </p>
        <table className="data-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Amount</th>
              <th>
                <Tooltip tip="Script type that locks the coins (how it can be spent).">
                  Script type
                </Tooltip>
              </th>
              <th>
                <Tooltip tip="Recipient address, or data when the output is OP_RETURN.">
                  Address / Data
                </Tooltip>
              </th>
              <th>
                <Tooltip tip="Readable form of the locking script (assembly-like opcodes).">
                  Script (ASM)
                </Tooltip>
              </th>
            </tr>
          </thead>
          <tbody>
            {report.vout.map((out) => (
              <tr key={out.n}>
                <td>{out.n}</td>
                <td style={{ fontFamily: "var(--mono)" }}>
                  {sats(out.value_sats)}
                </td>
                <td>
                  <ScriptTypeBadge scriptType={out.script_type} />
                </td>
                <td>
                  {out.address ? (
                    <span className="hex" aria-label="Recipient address">
                      {out.address}
                    </span>
                  ) : out.op_return_data_utf8 ? (
                    <span style={{ color: "var(--text2)", fontSize: 12 }}>
                      "{String(out.op_return_data_utf8).slice(0, 60)}"
                    </span>
                  ) : (
                    <span style={{ color: "var(--text2)" }}>—</span>
                  )}
                  {out.op_return_protocol &&
                    out.op_return_protocol !== "unknown" && (
                      <span
                        className="badge badge-sw"
                        style={{ marginLeft: 6 }}
                      >
                        {out.op_return_protocol}
                      </span>
                    )}
                </td>
                <td>
                  <span className="hex">
                    {out.script_asm.slice(0, 80)}
                    {out.script_asm.length > 80 ? "…" : ""}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Section>

      <div style={{ textAlign: "center" }}>
        <button className="detail-btn" onClick={() => setShowTech(!showTech)}>
          {showTech
            ? "▲ Hide technical details"
            : "▼ Show technical details (hex, scripts, etc.)"}
        </button>
      </div>

      {showTech && (
        <Section title="🔬 Technical Details" defaultOpen={true}>
          <table className="data-table">
            <tbody>
              <tr>
                <td style={{ color: "var(--text2)", width: 160 }}>
                  <Tooltip tip="Unique identifier for this transaction.">
                    Transaction ID (txid)
                  </Tooltip>
                </td>
                <td>
                  <span className="hex">{report.txid}</span>
                </td>
              </tr>
              {report.wtxid && (
                <tr>
                  <td style={{ color: "var(--text2)" }}>
                    <Tooltip tip="Identifier that includes witness data (SegWit-only).">
                      Witness ID (wtxid)
                    </Tooltip>
                  </td>
                  <td>
                    <span className="hex">{report.wtxid}</span>
                  </td>
                </tr>
              )}
              <tr>
                <td style={{ color: "var(--text2)" }}>
                  <Tooltip tip="Transaction format version.">Version</Tooltip>
                </td>
                <td>{report.version}</td>
              </tr>
              <tr>
                <td style={{ color: "var(--text2)" }}>
                  <Tooltip tip="When this transaction becomes valid by block height or time.">
                    Locktime
                  </Tooltip>
                </td>
                <td>
                  {report.locktime} ({report.locktime_type})
                </td>
              </tr>
              <tr>
                <td style={{ color: "var(--text2)" }}>
                  <Tooltip tip="Total size in bytes.">Size</Tooltip>
                </td>
                <td>{report.size_bytes} bytes</td>
              </tr>
              <tr>
                <td style={{ color: "var(--text2)" }}>
                  <Tooltip tip="Weight units. Witness data counts less.">
                    Weight
                  </Tooltip>
                </td>
                <td>{report.weight} WU</td>
              </tr>
              <tr>
                <td style={{ color: "var(--text2)" }}>
                  <Tooltip tip="Virtual bytes: size used for fee calculation.">
                    Virtual size (vB)
                  </Tooltip>
                </td>
                <td>{report.vbytes}</td>
              </tr>
              <tr>
                <td style={{ color: "var(--text2)" }}>
                  <Tooltip tip="Replace-By-Fee flag that allows fee bumping.">
                    RBF signaling
                  </Tooltip>
                </td>
                <td>{report.rbf_signaling ? "Yes" : "No"}</td>
              </tr>
            </tbody>
          </table>
        </Section>
      )}
    </div>
  );
}
