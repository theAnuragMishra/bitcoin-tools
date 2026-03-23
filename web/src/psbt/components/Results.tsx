import type { BuildReport } from "../types";
import { formatBtc, formatSats, pct } from "../utils";
import InputsPanel from "./Inputs";
import OutputsPanel from "./Outputs";
import PropertiesPanel from "./Properties";
import PsbtPanel from "./PSBT";
import StatCard from "./StatCard";
import Tip from "./Tooltip";

const WARNING_INFO: Record<
  string,
  { severity: "info" | "warn" | "error"; icon: string; desc: string }
> = {
  HIGH_FEE: {
    severity: "error",
    icon: "!",
    desc: "Fee is unusually high -- over 1M sats or rate above 200 sat/vB",
  },
  DUST_CHANGE: {
    severity: "warn",
    icon: "~",
    desc: "Change output was too small (dust) and was absorbed into the fee",
  },
  SEND_ALL: {
    severity: "info",
    icon: "i",
    desc: "All funds are being sent -- no change output returned to you",
  },
  RBF_SIGNALING: {
    severity: "info",
    icon: "R",
    desc: "Transaction signals Replace-By-Fee -- it can be bumped later",
  },
  LOW_FEE_RATE: {
    severity: "warn",
    icon: "~",
    desc: "Fee rate is below 1 sat/vB -- transaction may not confirm in reasonable time",
  },
  MANY_INPUTS: {
    severity: "warn",
    icon: "#",
    desc: "More than 50 inputs selected -- high cost and complex transaction",
  },
  LARGE_TRANSACTION: {
    severity: "warn",
    icon: "L",
    desc: "Transaction exceeds 10,000 vBytes -- may face relay or confirmation issues",
  },
  UNECONOMICAL_INPUTS: {
    severity: "warn",
    icon: "$",
    desc: "Some inputs cost more to spend than they are worth at this fee rate",
  },
};

export default function ResultsPanel({ report }: { report: BuildReport }) {
  const totalIn = report.selected_inputs.reduce((s, i) => s + i.value_sats, 0);
  const totalPayments = report.outputs
    .filter((o) => !o.is_change)
    .reduce((s, o) => s + o.value_sats, 0);
  const changeAmount = report.outputs
    .filter((o) => o.is_change)
    .reduce((s, o) => s + o.value_sats, 0);
  const hasChange = changeAmount > 0;
  const privacyColor =
    report.privacy.risk_level === "high"
      ? "text-red-400"
      : report.privacy.risk_level === "medium"
        ? "text-amber-400"
        : "text-emerald-400";

  return (
    <div className="space-y-6">
      {/* ── Warnings Banner ─────────────────────────────────────── */}
      {report.warnings.length > 0 && (
        <div className="space-y-2">
          {report.warnings.map((w, i) => {
            const info = WARNING_INFO[w.code] || {
              severity: "info" as const,
              icon: "?",
              desc: w.code,
            };
            const colors = {
              error: "bg-red-500/10 border-red-500/20 text-red-300",
              warn: "bg-amber-500/10 border-amber-500/20 text-amber-300",
              info: "bg-sky-500/10 border-sky-500/20 text-sky-300",
            }[info.severity];
            const iconBg = {
              error: "bg-red-500/20 text-red-400",
              warn: "bg-amber-500/20 text-amber-400",
              info: "bg-sky-500/20 text-sky-400",
            }[info.severity];

            return (
              <div
                key={i}
                className={`${colors} border rounded-xl px-5 py-3.5 flex items-center gap-3`}
              >
                <div
                  className={`w-7 h-7 rounded-full ${iconBg} flex items-center justify-center shrink-0 text-xs font-bold`}
                >
                  {info.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <span className="font-mono text-xs font-bold mr-2">
                    {w.code}
                  </span>
                  <span className="text-sm opacity-80">{info.desc}</span>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* ── Flow Diagram ────────────────────────────────────────── */}
      <div className="bg-[#111827]/60 border border-white/5 rounded-2xl overflow-hidden">
        <div className="px-6 py-4 border-b border-white/5">
          <h3 className="text-sm font-semibold text-gray-200">
            Transaction Flow
          </h3>
          <p className="text-xs text-gray-500 mt-0.5">
            How your bitcoin moves from inputs through outputs
          </p>
        </div>
        <div className="p-6">
          {/* Three column flow: Inputs -> Fee -> Outputs */}
          <div className="grid grid-cols-[1fr_auto_1fr] gap-4 items-center">
            {/* Left: Inputs */}
            <div className="space-y-3">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-medium mb-1">
                Coins In
              </div>
              <div className="bg-emerald-500/5 border border-emerald-500/20 rounded-xl p-4">
                <div className="text-2xl font-bold text-emerald-400 font-mono">
                  {formatBtc(totalIn)}
                </div>
                <div className="text-xs text-emerald-400/60 mt-1">
                  BTC from {report.selected_inputs.length} input
                  {report.selected_inputs.length !== 1 ? "s" : ""}
                </div>
              </div>
            </div>

            {/* Center: Arrow + Fee */}

            <div className="flex flex-col items-center justify-center min-w-27.5 pt-7 px-1">
              {/* Top arrow line */}
              <div className="w-0.5 flex-1 min-h-5 my-1 bg-linear-to-b from-transparent via-amber-500 to-transparent" />

              <div className=" border border-amber-500 rounded-lg p-3 text-center min-w-22.5">
                <div className="text-[22px] text-amber-500 leading-none">₿</div>

                <div className="text-[10px] text-slate-400 mt-1 font-semibold uppercase">
                  Bitcoin Transaction
                </div>

                <Tip text="The fee is the difference between total inputs and total outputs. It is collected by the miner who includes this transaction in a block.">
                  <div className="mt-2 bg-amber-500/10 border border-dashed border-amber-500 rounded-md px-2 py-1.5 text-[11px] font-semibold text-amber-500 cursor-help">
                    <span className="relative inline-flex border-b border-dashed border-slate-400 text-white">
                      {formatSats(report.fee_sats)}
                    </span>

                    <div className="text-[11px] mt-1 text-white">
                      {report.fee_rate_sat_vb.toFixed(2)} sat/vB
                    </div>
                  </div>
                </Tip>
              </div>

              {/* Bottom arrow line */}
              <div className="w-0.5 flex-1 min-h-5 my-1 bg-linear-to-b from-transparent via-amber-500 to-transparent" />
            </div>

            {/* Right: Outputs */}
            <div className="space-y-3">
              <div className="text-xs text-gray-500 uppercase tracking-wider font-medium mb-1">
                Going Out
              </div>
              <div className="bg-blue-500/5 border border-blue-500/20 rounded-xl p-3">
                <div className="text-[10px] text-blue-400/50 uppercase tracking-wider mb-1">
                  Payments
                </div>
                <div className="text-xl font-bold text-blue-400 font-mono">
                  {formatBtc(totalPayments)}
                </div>
                <div className="text-[10px] text-blue-400/50 mt-0.5">
                  to {report.outputs.filter((o) => !o.is_change).length}{" "}
                  recipient
                  {report.outputs.filter((o) => !o.is_change).length !== 1
                    ? "s"
                    : ""}
                </div>
              </div>
              {hasChange && (
                <div className="bg-purple-500/5 border border-purple-500/20 rounded-xl p-3">
                  <div className="text-[10px] text-purple-400/50 uppercase tracking-wider mb-1">
                    Change Back
                  </div>
                  <div className="text-lg font-bold text-purple-400 font-mono">
                    {formatBtc(changeAmount)}
                  </div>
                  <div className="text-[10px] text-purple-400/50 mt-0.5">
                    returned to your wallet
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Proportion Bar */}
          <div className="mt-6">
            <div className="flex items-center justify-between mb-2">
              <div className="text-[10px] text-gray-500 uppercase tracking-wider">
                Breakdown of total input value
              </div>
              <div className="text-[10px] text-gray-500 font-mono">
                {formatSats(totalIn)} total
              </div>
            </div>
            <div className="h-3 overflow-hidden flex bg-gray-800">
              <div
                className=" bg-blue-500 transition-all"
                style={{
                  width: `${pct(totalPayments, totalIn)}%`,
                  minWidth: totalPayments > 0 ? "2px" : "0",
                }}
              >
                <Tip
                  text={`Payments: ${formatSats(totalPayments)} (${pct(totalPayments, totalIn)}%)`}
                >
                  <span className="inline-block w-full h-3"></span>
                </Tip>
              </div>
              {hasChange && (
                <div
                  className=" bg-purple-500 transition-all"
                  style={{
                    width: `${pct(changeAmount, totalIn)}%`,
                    minWidth: "2px",
                  }}
                >
                  <Tip
                    text={`Change: ${formatSats(changeAmount)} (${pct(changeAmount, totalIn)}%)`}
                  >
                    <span className="inline-block w-full h-3"></span>
                  </Tip>
                </div>
              )}
              <div
                className="bg-amber-500 transition-all"
                style={{
                  width: `${pct(report.fee_sats, totalIn)}%`,
                  minWidth: "2px",
                }}
              >
                <Tip
                  text={`Fee: ${formatSats(report.fee_sats)} (${pct(report.fee_sats, totalIn)}%)`}
                >
                  <span className="inline-block w-full h-3"></span>
                </Tip>
              </div>
            </div>
            <div className="flex gap-5 mt-2.5 text-[11px]">
              <span className="flex items-center gap-1.5 text-gray-400">
                <span className="w-2.5 h-2.5 rounded-sm bg-blue-500" />
                Payments &nbsp;
                <span className="font-mono text-blue-400">
                  {pct(totalPayments, totalIn)}%
                </span>
              </span>
              {hasChange && (
                <span className="flex items-center gap-1.5 text-gray-400">
                  <span className="w-2.5 h-2.5 rounded-sm bg-purple-500" />
                  Change &nbsp;
                  <span className="font-mono text-purple-400">
                    {pct(changeAmount, totalIn)}%
                  </span>
                </span>
              )}
              <span className="flex items-center gap-1.5 text-gray-400">
                <span className="w-2.5 h-2.5 rounded-sm bg-amber-500" />
                Fee &nbsp;
                <span className="font-mono text-amber-400">
                  {pct(report.fee_sats, totalIn)}%
                </span>
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* ── Stats Row ───────────────────────────────────────────── */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard
          label="Virtual Size"
          value={`${report.vbytes} vB`}
          hint="Transaction weight in virtual bytes -- determines fee cost"
        />
        <StatCard
          label="Fee Rate"
          value={`${report.fee_rate_sat_vb.toFixed(2)} sat/vB`}
          hint="Satoshis per virtual byte -- higher = faster confirmation"
        />
        <StatCard
          label="Strategy"
          value={report.strategy}
          hint="The algorithm used to select which coins (UTXOs) to spend"
        />
        <StatCard
          label="Network"
          value={report.network}
          hint="Bitcoin network -- mainnet is the real network with real value"
        />
      </div>

      {/* ── Details Grid ────────────────────────────────────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <InputsPanel report={report} />
        <OutputsPanel report={report} />
      </div>

      {/* ── Transaction Properties ──────────────────────────────── */}
      <PropertiesPanel report={report} />

      <div className="bg-[#111827]/60 border border-white/5 rounded-2xl overflow-hidden">
        <div className="px-6 py-4 border-b border-white/5">
          <h3 className="text-sm font-semibold text-gray-200">Privacy Meter</h3>
          <p className="text-xs text-gray-500 mt-0.5">
            Heuristic estimate of how easy it may be to link this transaction to
            your wallet
          </p>
        </div>
        <div className="p-6 space-y-4">
          <div className="flex items-end gap-4">
            <div className="text-4xl font-mono font-bold text-gray-100">
              {report.privacy.score}
            </div>
            <div className="text-xs text-gray-500 pb-1">
              / 100 privacy score
            </div>
            <div
              className={`text-xs uppercase tracking-wide pb-1 ${privacyColor}`}
            >
              {report.privacy.risk_level} linkage risk
            </div>
          </div>
          <div className="h-2 bg-gray-800 rounded-full overflow-hidden">
            <div
              className={`h-2 ${
                report.privacy.score >= 70
                  ? "bg-emerald-500"
                  : report.privacy.score >= 40
                    ? "bg-amber-500"
                    : "bg-red-500"
              }`}
              style={{ width: `${report.privacy.score}%` }}
            />
          </div>
          {report.privacy.factors.length > 0 && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
              {report.privacy.factors.map((factor, idx) => (
                <div
                  key={idx}
                  className="text-xs text-gray-300 bg-white/5 border border-white/10 rounded-lg px-3 py-2"
                >
                  {factor}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="bg-[#111827]/60 border border-white/5 rounded-2xl overflow-hidden">
        <div className="px-6 py-4 border-b border-white/5">
          <h3 className="text-sm font-semibold text-gray-200">
            Watch-Only Descriptors
          </h3>
          <p className="text-xs text-gray-500 mt-0.5">
            Descriptor export to monitor inputs and outputs without private keys
          </p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-0 divide-y md:divide-y-0 md:divide-x divide-white/5">
          <div className="p-4 space-y-2">
            <div className="text-xs uppercase tracking-wide text-gray-500">
              Input descriptors
            </div>
            {report.watch_only_descriptors.inputs.length === 0 ? (
              <div className="text-xs text-gray-500">None</div>
            ) : (
              report.watch_only_descriptors.inputs.map((d, idx) => (
                <pre
                  key={idx}
                  className="text-[11px] text-gray-300 bg-[#0a0e1a] border border-white/10 rounded-lg p-2 break-all whitespace-pre-wrap"
                >
                  {d}
                </pre>
              ))
            )}
          </div>
          <div className="p-4 space-y-2">
            <div className="text-xs uppercase tracking-wide text-gray-500">
              Output descriptors
            </div>
            {report.watch_only_descriptors.outputs.length === 0 ? (
              <div className="text-xs text-gray-500">None</div>
            ) : (
              report.watch_only_descriptors.outputs.map((d, idx) => (
                <pre
                  key={idx}
                  className="text-[11px] text-gray-300 bg-[#0a0e1a] border border-white/10 rounded-lg p-2 break-all whitespace-pre-wrap"
                >
                  {d}
                </pre>
              ))
            )}
          </div>
        </div>
      </div>

      {/* ── PSBT ────────────────────────────────────────────────── */}
      <PsbtPanel report={report} />
    </div>
  );
}
