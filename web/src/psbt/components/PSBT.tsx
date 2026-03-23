import { useState } from "react";
import type { BuildReport } from "../types";

export default function PsbtPanel({ report }: { report: BuildReport }) {
  const [copied, setCopied] = useState(false);
  const [showFull, setShowFull] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(report.psbt_base64);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const psbtDisplay = showFull
    ? report.psbt_base64
    : report.psbt_base64.length > 300
      ? report.psbt_base64.slice(0, 300) + "..."
      : report.psbt_base64;

  // const signedDisplay = report.signed_psbt_base64
  //   ? report.signed_psbt_base64.length > 220
  //     ? report.signed_psbt_base64.slice(0, 220) + "..."
  //     : report.signed_psbt_base64
  //   : null;

  return (
    <div className="bg-[#111827]/60 border border-white/5 rounded-2xl overflow-hidden">
      <div className="px-6 py-4 border-b border-white/5 flex items-center justify-between">
        <div>
          <h3 className="text-sm font-semibold text-gray-200">PSBT Output</h3>
          <p className="text-xs text-gray-500 mt-0.5">
            Partially Signed Bitcoin Transaction -- ready for signing by your
            wallet
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-[10px] text-gray-500 font-mono">
            {report.psbt_base64.length} chars | ~
            {Math.ceil(report.psbt_base64.length * 0.75)} bytes
          </span>
          <button
            onClick={handleCopy}
            className={`px-3 py-1.5 text-xs rounded-lg border transition-all ${
              copied
                ? "bg-emerald-500/20 border-emerald-500/30 text-emerald-400"
                : "bg-white/5 border-white/10 text-gray-400 hover:text-white hover:bg-white/10"
            }`}
          >
            {copied ? "Copied!" : "Copy Base64"}
          </button>
        </div>
      </div>
      <div className="p-4">
        <div className="bg-[#0a0e1a] rounded-xl p-4 border border-white/5">
          <pre className="font-mono text-[11px] text-gray-400 break-all whitespace-pre-wrap leading-relaxed">
            {psbtDisplay}
          </pre>
        </div>
        {report.psbt_base64.length > 300 && (
          <button
            onClick={() => setShowFull(!showFull)}
            className="mt-2 text-xs text-amber-400 hover:text-amber-300 transition-colors"
          >
            {showFull ? "Collapse" : "Show full PSBT"}
          </button>
        )}

        {/* <div className="mt-4 space-y-3">
          <div className="text-xs text-gray-500 uppercase tracking-wide">
            Test signing artifacts
          </div>

          {signedDisplay ? (
            <div className="bg-[#0a0e1a] rounded-xl p-3 border border-white/5">
              <div className="text-[10px] text-gray-500 mb-1">
                Signed PSBT (base64)
              </div>
              <pre className="font-mono text-[11px] text-emerald-300 break-all whitespace-pre-wrap leading-relaxed">
                {signedDisplay}
              </pre>
            </div>
          ) : (
            <div className="text-xs text-gray-500">
              No signed PSBT generated. Provide `test_signing_wifs` in fixture
              that match selected UTXO scripts.
            </div>
          )}

          {report.final_tx_hex ? (
            <div className="bg-[#0a0e1a] rounded-xl p-3 border border-white/5">
              <div className="text-[10px] text-gray-500 mb-1">
                Finalized transaction hex
              </div>
              <pre className="font-mono text-[11px] text-sky-300 break-all whitespace-pre-wrap leading-relaxed">
                {report.final_tx_hex}
              </pre>
            </div>
          ) : (
            <div className="text-xs text-gray-500">
              Final transaction hex unavailable until all selected inputs are
              successfully signed and finalized.
            </div>
          )}
        </div> */}
      </div>
    </div>
  );
}
