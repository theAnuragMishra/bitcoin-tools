import type { Dispatch, SetStateAction } from "react";

export default function TxInputCard({
  fixtureText,
  setFixtureText,
  exampleFixture,
  loading,
  analyzeTx,
}: {
  fixtureText: string;
  setFixtureText: Dispatch<SetStateAction<string>>;
  exampleFixture: string;
  loading: boolean;
  analyzeTx: () => void;
}) {
  return (
    <div className="input-card">
      <h2>Analyze a Bitcoin Transaction</h2>
      <p>
        Paste a transaction fixture JSON below. The fixture must include the raw
        transaction hex and prevout information (the outputs being spent).
      </p>
      <textarea
        value={fixtureText}
        onChange={(e) => setFixtureText(e.target.value)}
        placeholder={exampleFixture}
        spellCheck={false}
      />
      <div className="flex-row">
        <button
          className="btn btn-primary"
          onClick={analyzeTx}
          disabled={loading || !fixtureText.trim()}
        >
          {loading ? (
            <>
              <span className="spinner" /> Analyzing…
            </>
          ) : (
            "🔍 Analyze Transaction"
          )}
        </button>
        <button
          className="btn btn-secondary"
          onClick={() => setFixtureText(exampleFixture)}
        >
          ✨ Load sample transaction
        </button>
      </div>
    </div>
  );
}
