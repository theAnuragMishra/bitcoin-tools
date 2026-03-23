import type { Dispatch, SetStateAction } from "react";

export default function BlockInputCard({
  blkFile,
  revFile,
  xorFile,
  setBlkFile,
  setRevFile,
  setXorFile,
  loading,
  analyzeBlock,
}: {
  blkFile: File | null;
  revFile: File | null;
  xorFile: File | null;
  setBlkFile: Dispatch<SetStateAction<File | null>>;
  setRevFile: Dispatch<SetStateAction<File | null>>;
  setXorFile: Dispatch<SetStateAction<File | null>>;
  loading: boolean;
  analyzeBlock: () => void;
}) {
  return (
    <div className="input-card">
      <h2>Analyze Block Files</h2>
      <p>
        Upload Bitcoin Core block data files. The blk*.dat file contains raw
        blocks; rev*.dat contains the "undo" data (spent output info); xor.dat
        is the obfuscation key.
      </p>
      <div className="file-row">
        <label className={`file-label ${blkFile ? "has-file" : ""}`}>
          <input
            type="file"
            accept=".dat"
            onChange={(e) => setBlkFile(e.target.files?.[0] || null)}
          />
          {blkFile ? `✓ ${blkFile.name}` : "📁 blk*.dat"}
        </label>
        <label className={`file-label ${revFile ? "has-file" : ""}`}>
          <input
            type="file"
            accept=".dat"
            onChange={(e) => setRevFile(e.target.files?.[0] || null)}
          />
          {revFile ? `✓ ${revFile.name}` : "📁 rev*.dat"}
        </label>
        <label className={`file-label ${xorFile ? "has-file" : ""}`}>
          <input
            type="file"
            accept=".dat"
            onChange={(e) => setXorFile(e.target.files?.[0] || null)}
          />
          {xorFile ? `✓ ${xorFile.name}` : "📁 xor.dat"}
        </label>
      </div>
      <div className="flex-row">
        <button
          className="btn btn-primary"
          onClick={analyzeBlock}
          disabled={loading || !blkFile || !revFile || !xorFile}
        >
          {loading ? (
            <>
              <span className="spinner" /> Parsing…
            </>
          ) : (
            "⛏ Parse Block Files"
          )}
        </button>
      </div>
    </div>
  );
}
