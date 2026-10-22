import { useState, useRef } from "react";
import Tip from "./components/Tooltip";
import ResultsPanel from "./components/Results";
import type { ApiResponse, BuildReport, ErrorReport } from "./types";
import { API_BASE } from "../config";
import "./index.css";

const SAMPLE_FIXTURE = `{
  "network": "mainnet",
  "utxos": [
    {
      "txid": "d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1",
      "vout": 0,
      "value_sats": 500000,
      "script_pubkey_hex": "0014d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1",
      "script_type": "p2wpkh",
      "address": "bc1q68gar5w368gar5w368gar5w368gar5w32he3ks"
    },
{
      "txid": "d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1",
      "vout": 1,
      "value_sats": 200000,
      "script_pubkey_hex": "0014d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1",
      "script_type": "p2wpkh",
      "address": "bc1q68gar5w368gar5w368gar5w368gar5w32he3ks"
    },
{
      "txid": "d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1",
      "vout": 2,
      "value_sats": 300000,
      "script_pubkey_hex": "0014d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1",
      "script_type": "p2wpkh",
      "address": "bc1q68gar5w368gar5w368gar5w368gar5w32he3ks"
    }
  ],
  "payments": [
    {
      "address": "bc1q6tfd95kj6tfd95kj6tfd95kj6tfd95kjmnl5ax",
      "script_pubkey_hex": "0014d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2",
      "script_type": "p2wpkh",
      "value_sats": 700000
    }
  ],
  "change": {
    "address": "bc1q60fa857n60fa857n60fa857n60fa857n6r74h8",
    "script_pubkey_hex": "0014d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3",
    "script_type": "p2wpkh"
  },
  "fee_rate_sat_vb": 5,
  "rbf": true,
  "locktime": 870000,
  "policy": {
    "max_inputs": 5
  }
}

`;

function App() {
  const [fixtureJson, setFixtureJson] = useState(SAMPLE_FIXTURE);
  const [report, setReport] = useState<BuildReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [fixtureCollapsed, setFixtureCollapsed] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const handleBuild = async () => {
    setLoading(true);
    setError(null);
    setReport(null);
    try {
      const parsed = JSON.parse(fixtureJson);
      const res = await fetch(`${API_BASE}/api/build`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(parsed),
      });
      const data: ApiResponse = await res.json();
      if (data.ok) {
        setReport(data as BuildReport);
        setFixtureCollapsed(true);
      } else {
        const err = data as ErrorReport;
        setError(`${err.error.code}: ${err.error.message}`);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => setFixtureJson(ev.target?.result as string);
    reader.readAsText(file);
    setFixtureCollapsed(false);
  };

  return (
    <div className="min-h-screen bg-[#0a0e1a] text-gray-100 font-sans">
      {/* ── Header ─────────────────────────────────────────────────── */}
      <header className="border-b border-white/5 bg-[#0d1225]/80 backdrop-blur-md sticky top-0 z-40">
        <div className="mx-auto max-w-7xl px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-linear-to-br from-amber-400 to-orange-500 flex items-center justify-center shadow-lg shadow-amber-500/20">
              <svg
                className="w-5 h-5 text-black"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <circle cx="12" cy="12" r="10" />
                <path d="M12 6v12M9 9h4.5a2.5 2.5 0 010 5H9" />
              </svg>
            </div>
            <div>
              <h1 className="text-lg font-bold text-white tracking-tight">
                Coin Smith
              </h1>
              <p className="text-[10px] text-gray-500 -mt-0.5 tracking-wide uppercase">
                PSBT Builder
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Tip text="Partially Signed Bitcoin Transaction format (BIP-174)">
              <span className="px-2.5 py-1 bg-white/5 border border-white/10 rounded-md text-[11px] text-gray-400 font-mono">
                BIP-174
              </span>
            </Tip>
            <Tip text="Bitcoin Mainnet -- real bitcoin network">
              <span className="px-2.5 py-1 bg-white/5 border border-white/10 rounded-md text-[11px] text-gray-400 font-mono">
                {report?.network}
              </span>
            </Tip>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-7xl px-6 py-8 space-y-8">
        {/* ── Input Section ────────────────────────────────────────── */}
        <section className="bg-[#111827]/60 border border-white/5 rounded-2xl overflow-hidden">
          <div className="px-6 py-4 border-b border-white/5 flex items-center justify-between">
            <div>
              <h2 className="text-sm font-semibold text-gray-200">
                Transaction Fixture
              </h2>
              <p className="text-xs text-gray-500 mt-0.5">
                Paste JSON describing your UTXOs, payments, and fee rate
              </p>
            </div>
            <div className="flex items-center gap-3">
              <button
                onClick={() => fileRef.current?.click()}
                className="px-3 py-1.5 text-xs text-gray-400 hover:text-white bg-white/5 hover:bg-white/10 border border-white/10 rounded-lg transition-all"
              >
                Upload JSON
              </button>
              <input
                ref={fileRef}
                type="file"
                accept=".json"
                className="hidden"
                onChange={handleFileUpload}
              />
              {(report || error) && (
                <button
                  onClick={() => setFixtureCollapsed(!fixtureCollapsed)}
                  className="px-3 py-1.5 text-xs text-gray-400 hover:text-white bg-white/5 hover:bg-white/10 border border-white/10 rounded-lg transition-all flex items-center gap-1.5"
                >
                  <svg
                    className={`w-3.5 h-3.5 transition-transform ${fixtureCollapsed ? "" : "rotate-180"}`}
                    viewBox="0 0 20 20"
                    fill="currentColor"
                  >
                    <path
                      fillRule="evenodd"
                      d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                      clipRule="evenodd"
                    />
                  </svg>
                  {fixtureCollapsed ? "Show fixture" : "Hide fixture"}
                </button>
              )}
            </div>
          </div>
          {!fixtureCollapsed && (
            <>
              <div className="p-4">
                <textarea
                  className="w-full h-64 bg-[#0a0e1a] border border-white/10 rounded-xl p-4 font-mono text-[13px] text-gray-300 focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/30 focus:outline-none resize-none leading-relaxed"
                  value={fixtureJson}
                  onChange={(e) => setFixtureJson(e.target.value)}
                  spellCheck={false}
                  placeholder="Paste your fixture JSON here..."
                />
              </div>
              <div className="px-4 pb-4">
                <button
                  onClick={handleBuild}
                  disabled={loading}
                  className="w-full py-3.5 bg-linear-to-r from-amber-500 to-orange-500 hover:from-amber-400 hover:to-orange-400 disabled:from-gray-700 disabled:to-gray-700 disabled:text-gray-500 text-black font-bold rounded-xl transition-all shadow-lg shadow-amber-500/20 hover:shadow-amber-500/30 disabled:shadow-none text-sm tracking-wide"
                >
                  {loading ? "Building Transaction..." : "Build Transaction"}
                </button>
              </div>
            </>
          )}
          {fixtureCollapsed && (
            <div className="px-6 py-3 flex items-center justify-between">
              <span className="text-xs text-gray-500 font-mono truncate max-w-lg">
                {fixtureJson.trim().slice(0, 120)}
                {fixtureJson.trim().length > 120 ? "…" : ""}
              </span>
              <button
                onClick={handleBuild}
                disabled={loading}
                className="ml-4 shrink-0 px-4 py-2 bg-linear-to-r from-amber-500 to-orange-500 hover:from-amber-400 hover:to-orange-400 disabled:from-gray-700 disabled:to-gray-700 disabled:text-gray-500 text-black font-bold rounded-lg transition-all text-xs tracking-wide"
              >
                {loading ? "Building…" : "Rebuild"}
              </button>
            </div>
          )}
        </section>

        {/* ── Error ────────────────────────────────────────────────── */}
        {error && (
          <div className="bg-red-500/10 border border-red-500/20 rounded-2xl p-5 flex items-start gap-3">
            <div className="w-8 h-8 rounded-full bg-red-500/20 flex items-center justify-center shrink-0 mt-0.5">
              <span className="text-red-400 font-bold text-sm">!</span>
            </div>
            <div>
              <p className="text-red-300 font-medium text-sm">Build Failed</p>
              <p className="text-red-400/70 text-sm font-mono mt-1">{error}</p>
            </div>
          </div>
        )}

        {/* ── Results ──────────────────────────────────────────────── */}
        {report && <ResultsPanel report={report} />}

        {/* ── Empty State ──────────────────────────────────────────── */}
        {!report && !error && (
          <div className="bg-[#111827]/40 border border-white/5 rounded-2xl p-16 flex flex-col items-center text-center">
            <div className="w-20 h-20 rounded-2xl bg-linear-to-br from-amber-500/10 to-orange-500/10 border border-amber-500/20 flex items-center justify-center mb-5">
              <svg
                className="w-9 h-9 text-amber-500/50"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M12 2L2 7l10 5 10-5-10-5z" />
                <path d="M2 17l10 5 10-5" />
                <path d="M2 12l10 5 10-5" />
              </svg>
            </div>
            <h3 className="text-gray-300 font-medium mb-1">
              No Transaction Built Yet
            </h3>
            <p className="text-gray-500 text-sm max-w-sm">
              Enter a fixture with your available coins (UTXOs), desired
              payments, and fee rate, then click Build to construct an unsigned
              PSBT.
            </p>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
