import { useState } from "react";
import type { BuildReport } from "../types";
import { formatBtc, formatSats, pct, truncateHash } from "../utils";
import Tip from "./Tooltip";

const SCRIPT_TYPE_INFO: Record<
  string,
  { label: string; desc: string; color: string }
> = {
  p2pkh: {
    label: "P2PKH",
    desc: "Legacy address -- Pay to Public Key Hash",
    color: "bg-orange-500/20 text-orange-300 border-orange-500/30",
  },
  p2wpkh: {
    label: "P2WPKH",
    desc: "Native SegWit -- Most efficient standard type",
    color: "bg-sky-500/20 text-sky-300 border-sky-500/30",
  },
  "p2sh-p2wpkh": {
    label: "P2SH-P2WPKH",
    desc: "Wrapped SegWit -- SegWit inside a legacy wrapper",
    color: "bg-violet-500/20 text-violet-300 border-violet-500/30",
  },
  p2tr: {
    label: "P2TR",
    desc: "Taproot -- Newest, most private and efficient",
    color: "bg-emerald-500/20 text-emerald-300 border-emerald-500/30",
  },
  p2wsh: {
    label: "P2WSH",
    desc: "SegWit Script Hash -- For complex scripts",
    color: "bg-pink-500/20 text-pink-300 border-pink-500/30",
  },
  p2sh: {
    label: "P2SH",
    desc: "Script Hash -- Legacy multisig / scripts",
    color: "bg-amber-500/20 text-amber-300 border-amber-500/30",
  },
};

export default function OutputsPanel({ report }: { report: BuildReport }) {
  const total = report.outputs.reduce((s, o) => s + o.value_sats, 0);
  const [expanded, setExpanded] = useState(false);
  const displayLimit = 5;
  const hasMore = report.outputs.length > displayLimit;
  const displayed = expanded
    ? report.outputs
    : report.outputs.slice(0, displayLimit);

  return (
    <div className="bg-[#111827]/60 border border-white/5 rounded-2xl overflow-hidden">
      <div className="px-5 py-4 border-b border-white/5 flex items-center justify-between">
        <div>
          <h3 className="text-sm font-semibold text-gray-200">
            Outputs
            <span className="ml-2 text-xs text-gray-500 font-normal">
              ({report.outputs.length})
            </span>
          </h3>
          <p className="text-[11px] text-gray-500 mt-0.5">
            Where the bitcoin is being sent
          </p>
        </div>
        <div className="text-right">
          <div className="text-sm font-bold text-blue-400 font-mono">
            {formatSats(total)}
          </div>
          <div className="text-[10px] text-gray-500">
            {formatBtc(total)} BTC
          </div>
        </div>
      </div>
      <div className="divide-y divide-white/5">
        {displayed.map((output, i) => (
          <div
            key={i}
            className={`px-5 py-3 flex items-center gap-4 transition-colors ${output.is_change ? "bg-purple-500/3 hover:bg-purple-500/5" : "hover:bg-white/2"}`}
          >
            <div
              className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 ${output.is_change ? "bg-purple-500/10" : "bg-blue-500/10"}`}
            >
              <span
                className={`text-xs font-mono font-bold ${output.is_change ? "text-purple-400" : "text-blue-400"}`}
              >
                {i}
              </span>
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-0.5 flex-wrap">
                <code
                  className="text-xs text-gray-400 font-mono truncate max-w-45"
                  title={output.script_pubkey_hex}
                >
                  {truncateHash(output.script_pubkey_hex, 10)}
                </code>
                {output.is_change && (
                  <Tip text="This output returns leftover funds back to your wallet">
                    <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-purple-500/20 text-purple-300 border border-purple-500/30">
                      CHANGE
                    </span>
                  </Tip>
                )}
                {!output.is_change && (
                  <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-blue-500/20 text-blue-300 border border-blue-500/30">
                    PAYMENT
                  </span>
                )}
                {(() => {
                  const info = SCRIPT_TYPE_INFO[output.script_type] || {
                    label: output.script_type || "?",
                    desc: "Script type",
                    color: "bg-gray-500/20 text-gray-300 border-gray-500/30",
                  };
                  return (
                    <Tip text={info.desc}>
                      <span
                        className={`px-1.5 py-0.5 rounded text-[10px] font-mono border ${info.color}`}
                      >
                        {info.label}
                      </span>
                    </Tip>
                  );
                })()}
              </div>
              <div className="h-1 rounded-full bg-gray-800 mt-1.5 w-full max-w-30">
                <div
                  className={`h-1 rounded-full ${output.is_change ? "bg-purple-500/60" : "bg-blue-500/60"}`}
                  style={{
                    width: `${pct(output.value_sats, total)}%`,
                    minWidth: "2px",
                  }}
                />
              </div>
            </div>
            <div className="text-right shrink-0">
              <div
                className={`text-sm font-medium font-mono ${output.is_change ? "text-purple-400" : "text-blue-400"}`}
              >
                {formatSats(output.value_sats)}
              </div>
              <div className="text-[10px] text-gray-500">
                {pct(output.value_sats, total)}%
              </div>
            </div>
          </div>
        ))}
      </div>
      {hasMore && (
        <div className="px-5 py-3 border-t border-white/5">
          <button
            onClick={() => setExpanded(!expanded)}
            className="text-xs text-amber-400 hover:text-amber-300 transition-colors"
          >
            {expanded
              ? "Show less"
              : `Show all ${report.outputs.length} outputs`}
          </button>
        </div>
      )}
    </div>
  );
}
