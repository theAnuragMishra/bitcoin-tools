import type { BuildReport } from "../types";
import Tip from "./Tooltip";

const LOCKTIME_DESC: Record<string, string> = {
  none: "No time restriction -- transaction can be mined immediately",
  block_height: "Locked until a specific block height is reached",
  unix_timestamp: "Locked until a specific date/time (Unix timestamp)",
};

export default function PropertiesPanel({ report }: { report: BuildReport }) {
  return (
    <div className="bg-[#111827]/60 border border-white/5 rounded-2xl overflow-hidden">
      <div className="px-6 py-4 border-b border-white/5">
        <h3 className="text-sm font-semibold text-gray-200">
          Transaction Properties
        </h3>
        <p className="text-xs text-gray-500 mt-0.5">
          Advanced details about how this transaction behaves on the network
        </p>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-3 divide-y md:divide-y-0 md:divide-x divide-white/5">
        {/* RBF */}
        <div className="p-5">
          <Tip text="Replace-By-Fee allows you to increase the fee later to speed up confirmation">
            <div className="flex items-center gap-2 mb-2 cursor-help">
              <div
                className={`w-4 h-4 rounded-full flex items-center justify-center shrink-0 ${
                  report.rbf_signaling ? "bg-amber-500/20" : "bg-gray-700/50"
                }`}
              >
                <svg
                  className={`w-3 h-3 ${report.rbf_signaling ? "text-amber-400" : "text-gray-500"}`}
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fillRule="evenodd"
                    d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z"
                    clipRule="evenodd"
                  />
                </svg>
              </div>
              <span className="text-xs text-gray-400 uppercase tracking-wider font-medium">
                RBF Signaling
              </span>
            </div>
          </Tip>
          <div
            className={`text-lg font-bold ${report.rbf_signaling ? "text-amber-400" : "text-gray-400"}`}
          >
            {report.rbf_signaling ? "Enabled" : "Disabled"}
          </div>
          <p className="text-[11px] text-gray-500 mt-1">
            {report.rbf_signaling
              ? "This transaction can be replaced with a higher-fee version"
              : "This transaction is final and cannot be fee-bumped"}
          </p>
        </div>

        {/* Locktime */}
        <div className="p-5">
          <Tip
            text={
              LOCKTIME_DESC[report.locktime_type] ||
              "Controls when this transaction can be mined"
            }
          >
            <div className="flex items-center gap-2 mb-2 cursor-help">
              <div
                className={`w-4 h-4 rounded-full flex items-center justify-center shrink-0 ${
                  report.locktime > 0 ? "bg-sky-500/20" : "bg-gray-700/50"
                }`}
              >
                <svg
                  className={`w-4 h-4 ${report.locktime > 0 ? "text-sky-400" : "text-gray-500"}`}
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z"
                    clipRule="evenodd"
                  />
                </svg>
              </div>
              <span className="text-xs text-gray-400 uppercase tracking-wider font-medium">
                Locktime
              </span>
            </div>
          </Tip>
          <div className="text-lg font-bold text-gray-200 font-mono">
            {report.locktime.toLocaleString()}
          </div>
          <div className="mt-1 flex items-center gap-1.5">
            <span
              className={`px-2 py-0.5 rounded text-[10px] font-mono ${
                report.locktime_type === "none"
                  ? "bg-gray-500/20 text-gray-400"
                  : report.locktime_type === "block_height"
                    ? "bg-sky-500/20 text-sky-300"
                    : "bg-indigo-500/20 text-indigo-300"
              }`}
            >
              {report.locktime_type}
            </span>
          </div>
        </div>

        {/* Change Index */}
        <div className="p-5">
          <Tip text="The position of the change output in the transaction, or null if no change">
            <div className="flex items-center gap-2 mb-2 cursor-help">
              <div
                className={`w-4 h-4 rounded-full flex items-center justify-center shrink-0 ${
                  report.change_index !== null
                    ? "bg-purple-500/20"
                    : "bg-gray-700/50"
                }`}
              >
                <svg
                  className={`w-4 h-4 ${report.change_index !== null ? "text-purple-400" : "text-gray-500"}`}
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path d="M8.433 7.418c.155-.103.346-.196.567-.267v1.698a2.305 2.305 0 01-.567-.267C8.07 8.34 8 8.114 8 8c0-.114.07-.34.433-.582zM11 12.849v-1.698c.22.071.412.164.567.267.364.243.433.468.433.582 0 .114-.07.34-.433.582a2.305 2.305 0 01-.567.267z" />
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-13a1 1 0 10-2 0v.092a4.535 4.535 0 00-1.676.662C6.602 6.234 6 7.009 6 8c0 .99.602 1.765 1.324 2.246.48.32 1.054.545 1.676.662v1.941c-.391-.127-.68-.317-.843-.504a1 1 0 10-1.51 1.31c.562.649 1.413 1.076 2.353 1.253V15a1 1 0 102 0v-.092a4.535 4.535 0 001.676-.662C13.398 13.766 14 12.991 14 12c0-.99-.602-1.765-1.324-2.246A4.535 4.535 0 0011 9.092V7.151c.391.127.68.317.843.504a1 1 0 101.511-1.31c-.563-.649-1.413-1.076-2.354-1.253V5z"
                    clipRule="evenodd"
                  />
                </svg>
              </div>
              <span className="text-xs text-gray-400 uppercase tracking-wider font-medium">
                Change Output
              </span>
            </div>
          </Tip>
          <div className="text-lg font-bold text-gray-200 font-mono">
            {report.change_index !== null
              ? `Index ${report.change_index}`
              : "None"}
          </div>
          <p className="text-[11px] text-gray-500 mt-1">
            {report.change_index !== null
              ? "Leftover funds are returned to your wallet"
              : "All input value goes to payments and fee (send-all)"}
          </p>
        </div>
      </div>
    </div>
  );
}
