import { useState } from "react";
import type { BlockReport } from "../types";
import Tooltip from "./Tooltip";
import ScriptTypeBadge from "./ScriptTypeBadge";
import TxVisualizer from "./TxVisualizer";
import { sats, shortHex } from "../utils/format";

export default function BlockVisualizer({
  reports,
}: {
  reports: BlockReport[];
}) {
  const [openBlock, setOpenBlock] = useState<number | null>(0);
  const [openTx, setOpenTx] = useState<number | null>(null);

  return (
    <div className="block-list">
      {reports.map((block, bi) => (
        <div className="block-entry" key={bi}>
          <div
            className="block-entry-hdr"
            onClick={() => setOpenBlock(openBlock === bi ? null : bi)}
          >
            <div>
              <strong>
                Block #{block.coinbase.bip34_height.toLocaleString()}
              </strong>
              <span
                style={{ color: "var(--text2)", marginLeft: 8, fontSize: 12 }}
              >
                {block.tx_count} txs · {sats(block.block_stats.total_fees_sats)}{" "}
                fees
              </span>
              <div className="bh">
                <span className="bh-label">Block hash</span>
                {block.block_header.block_hash}
              </div>
            </div>
            <span style={{ color: "var(--text2)" }}>
              {openBlock === bi ? "▲" : "▼"}
            </span>
          </div>
          {openBlock === bi && (
            <div style={{ padding: 16 }}>
              <div className="story-cards" style={{ marginBottom: 16 }}>
                <div className="story-card">
                  <div className="s-label">Height</div>
                  <div className="s-value">
                    {block.coinbase.bip34_height.toLocaleString()}
                  </div>
                </div>
                <div className="story-card">
                  <div className="s-label">Transactions</div>
                  <div className="s-value">{block.tx_count}</div>
                </div>
                <div className="story-card">
                  <div className="s-label">Total Fees</div>
                  <div className="s-value v-accent">
                    {sats(block.block_stats.total_fees_sats)}
                  </div>
                </div>
                <div className="story-card">
                  <div className="s-label">Avg Fee Rate</div>
                  <div className="s-value">
                    {block.block_stats.avg_fee_rate_sat_vb}
                  </div>
                  <div className="s-sub">sat/vB</div>
                </div>
                <div className="story-card">
                  <div className="s-label">Total Weight</div>
                  <div className="s-value">
                    {(block.block_stats.total_weight / 1e6).toFixed(2)}M
                  </div>
                  <div className="s-sub">WU</div>
                </div>
                <div className="story-card">
                  <div className="s-label">
                    <Tooltip tip="Checks whether the merkle root matches the transaction list.">
                      Merkle root check
                    </Tooltip>
                  </div>
                  <div
                    className={`s-value ${block.block_header.merkle_root_valid ? "v-green" : "v-red"}`}
                  >
                    {block.block_header.merkle_root_valid
                      ? "✓ Valid"
                      : "✗ Invalid"}
                  </div>
                </div>
              </div>
              <div
                style={{
                  marginBottom: 16,
                  background: "var(--surface2)",
                  borderRadius: "var(--radius)",
                  padding: 12,
                }}
              >
                <div
                  style={{
                    fontSize: 12,
                    color: "var(--text2)",
                    marginBottom: 8,
                  }}
                >
                  Output script mix in this block:
                </div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                  {Object.entries(block.block_stats.script_type_summary)
                    .sort((a, b) => b[1] - a[1])
                    .map(([type, count]) => (
                      <span key={type} className="script-chip">
                        <ScriptTypeBadge scriptType={type} />
                        <strong>{count}</strong>
                      </span>
                    ))}
                </div>
              </div>
              <div
                style={{ fontSize: 13, color: "var(--text2)", marginBottom: 8 }}
              >
                Transactions (click to expand):
              </div>
              {block.transactions.map((tx, ti) => (
                <div
                  key={ti}
                  style={{
                    marginBottom: 6,
                    border: "1px solid var(--border)",
                    borderRadius: "var(--radius)",
                    overflow: "hidden",
                  }}
                >
                  <div
                    style={{
                      padding: "8px 12px",
                      cursor: "pointer",
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      background: "var(--surface2)",
                    }}
                    onClick={() =>
                      setOpenTx(
                        openTx === ti + bi * 10000 ? null : ti + bi * 10000,
                      )
                    }
                  >
                    <div>
                      <span style={{ color: "var(--text2)", fontSize: 11 }}>
                        {ti === 0 ? "★ coinbase" : `#${ti}`}
                      </span>
                      <Tooltip tip={`Transaction ID: ${tx.txid}`}>
                        <span
                          className="hex"
                          style={{ marginLeft: 8, fontSize: 11 }}
                        >
                          tx {shortHex(tx.txid, 8)}
                        </span>
                      </Tooltip>
                      <span style={{ marginLeft: 8, fontSize: 12 }}>
                        {tx.vin.length}→{tx.vout.length}
                      </span>
                      {ti > 0 && (
                        <span
                          style={{
                            marginLeft: 8,
                            fontSize: 12,
                            color: "var(--accent)",
                          }}
                        >
                          {sats(tx.fee_sats)} fee
                        </span>
                      )}
                    </div>
                    <span style={{ fontSize: 12, color: "var(--text2)" }}>
                      {openTx === ti + bi * 10000 ? "▲" : "▼"}
                    </span>
                  </div>
                  {openTx === ti + bi * 10000 && (
                    <div style={{ padding: 16 }}>
                      <TxVisualizer report={tx} />
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
