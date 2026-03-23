import { useState } from "react";
import "./index.css";
import type { TxReport, BlockReport } from "./types";
import AppHeader from "./components/AppHeader";
import TxInputCard from "./components/TxInputCard";
import BlockInputCard from "./components/BlockInputCard";
import TxVisualizer from "./components/TxVisualizer";
import BlockVisualizer from "./components/BlockVisualizer";

export default function App() {
  const [tab, setTab] = useState<"tx" | "block">("tx");
  const [loadingTx, setLoadingTx] = useState(false);
  const [loadingBlk, setLoadingBlk] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [txReport, setTxReport] = useState<TxReport | null>(null);
  const [blockReports, setBlockReports] = useState<BlockReport[] | null>(null);

  const [fixtureText, setFixtureText] = useState("");

  const [blkFile, setBlkFile] = useState<File | null>(null);
  const [revFile, setRevFile] = useState<File | null>(null);
  const [xorFile, setXorFile] = useState<File | null>(null);

  async function analyzeTx() {
    setLoadingTx(true);
    setError(null);
    setTxReport(null);
    try {
      let body: Record<string, unknown>;
      try {
        body = { fixture_json: JSON.parse(fixtureText) };
      } catch {
        body = { fixture: fixtureText };
      }
      const res = await fetch("/api/analyze", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!data.ok) {
        setError(`${data.error?.code}: ${data.error?.message}`);
      } else {
        setTxReport(data as TxReport);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingTx(false);
    }
  }

  async function analyzeBlock() {
    if (!blkFile || !revFile || !xorFile) {
      setError(
        "Please select all three files: blk*.dat, rev*.dat, and xor.dat",
      );
      return;
    }
    setLoadingBlk(true);
    setError(null);
    setBlockReports(null);
    try {
      const formData = new FormData();
      formData.append("blk", blkFile);
      formData.append("rev", revFile);
      formData.append("xor", xorFile);
      const res = await fetch("/api/analyze/upload", {
        method: "POST",
        body: formData,
      });
      const data = await res.json();
      if (!data.ok) {
        setError(`${data.error?.code}: ${data.error?.message}`);
      } else {
        setBlockReports(data.blocks as BlockReport[]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingBlk(false);
    }
  }

  const exampleFixture = `{
  "network": "mainnet",
  "raw_tx": "02000000000102a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a10000000000feffffffb2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b20200000000fdffffff0210270000000000001600141313131313131313131313131313131313131313e02e00000000000022512014141414141414141414141414141414141414141414141414141414141414140247eed60c73dec1a4307bb6ade00521a58737c9e184e5f42329a9d82f63456c886ade14624e2f582c43767fffc7d06a49772d53847b711e86cb86cb46d018d9363b01f417e074a55421688bc8d5d9d8ab2ef6065ad8285a8faf92ca89371c9eb7478ec091bb36914b90440000000000",
  "prevouts": [
    {
      "txid": "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",
      "vout": 0,
      "value_sats": 15000,
      "script_pubkey_hex": "00141515151515151515151515151515151515151515"
    },
    {
      "txid": "b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2",
      "vout": 2,
      "value_sats": 190000,
      "script_pubkey_hex": "76a914161616161616161616161616161616161616161688ac"
    }
  ]
}`;

  return (
    <div className="app">
      <AppHeader />
      <main>
        <div className="tabs">
          <button
            className={`tab-btn ${tab === "tx" ? "active" : ""}`}
            onClick={() => {
              setTab("tx");
              setError(null);
            }}
          >
            Transaction
          </button>
          <button
            className={`tab-btn ${tab === "block" ? "active" : ""}`}
            onClick={() => {
              setTab("block");
              setError(null);
            }}
          >
            Block Files
          </button>
        </div>

        {tab === "tx" && (
          <TxInputCard
            fixtureText={fixtureText}
            setFixtureText={setFixtureText}
            exampleFixture={exampleFixture}
            loading={loadingTx}
            analyzeTx={analyzeTx}
          />
        )}

        {tab === "block" && (
          <BlockInputCard
            blkFile={blkFile}
            revFile={revFile}
            xorFile={xorFile}
            setBlkFile={setBlkFile}
            setRevFile={setRevFile}
            setXorFile={setXorFile}
            loading={loadingBlk}
            analyzeBlock={analyzeBlock}
          />
        )}

        {error && <div className="error-box">⚠ {error}</div>}

        {tab === "tx" && txReport && <TxVisualizer report={txReport} />}
        {tab === "block" && blockReports && (
          <BlockVisualizer reports={blockReports} />
        )}
      </main>
    </div>
  );
}
