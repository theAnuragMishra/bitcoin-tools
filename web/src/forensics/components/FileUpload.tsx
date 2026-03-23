import { Upload, FileText, CheckCircle, Loader2 } from 'lucide-react';
import { useRef } from 'react';

interface FileUploadProps {
  blkFile: File | null;
  revFile: File | null;
  xorFile: File | null;
  onBlkChange: (f: File | null) => void;
  onRevChange: (f: File | null) => void;
  onXorChange: (f: File | null) => void;
  onAnalyze: () => void;
  loading: boolean;
  error: string | null;
}

function FileInput({
  label,
  hint,
  file,
  onChange,
  accept,
}: {
  label: string;
  hint: string;
  file: File | null;
  onChange: (f: File | null) => void;
  accept?: string;
}) {
  const ref = useRef<HTMLInputElement>(null);
  const selected = file !== null;

  return (
    <button
      type="button"
      onClick={() => ref.current?.click()}
      className={`relative flex flex-col items-center justify-center p-6 rounded-xl border-2 border-dashed transition-all duration-300 cursor-pointer min-h-[140px] ${
        selected
          ? 'border-btc-green/50 bg-btc-green/5'
          : 'border-btc-border hover:border-btc-orange/40 hover:bg-btc-card-hover bg-btc-card'
      }`}
    >
      <input
        ref={ref}
        type="file"
        className="hidden"
        accept={accept}
        onChange={(e) => onChange(e.target.files?.[0] ?? null)}
      />
      {selected ? (
        <>
          <CheckCircle className="text-btc-green mb-2" size={28} />
          <span className="text-btc-green font-medium text-sm">{label}</span>
          <span className="text-btc-muted text-xs mt-1 font-mono truncate max-w-full px-2">
            {file.name}
          </span>
          <span className="text-btc-muted/60 text-xs mt-0.5">
            {(file.size / 1024 / 1024).toFixed(1)} MB
          </span>
        </>
      ) : (
        <>
          <Upload className="text-btc-muted mb-2" size={28} />
          <span className="text-btc-text font-medium text-sm">{label}</span>
          <span className="text-btc-muted text-xs mt-1">{hint}</span>
        </>
      )}
    </button>
  );
}

export default function FileUpload({
  blkFile,
  revFile,
  xorFile,
  onBlkChange,
  onRevChange,
  onXorChange,
  onAnalyze,
  loading,
  error,
}: FileUploadProps) {
  const allSelected = blkFile && revFile && xorFile;

  return (
    <div className="bg-btc-card border border-btc-border rounded-2xl p-8">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-xl bg-btc-orange/10 flex items-center justify-center">
          <FileText className="text-btc-orange" size={20} />
        </div>
        <div>
          <h2 className="text-lg font-semibold text-btc-text">
            Upload Block Data
          </h2>
          <p className="text-btc-muted text-sm">
            Select the three required files from Bitcoin Core's data directory
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <FileInput
          label="Block Data"
          hint="blk*.dat"
          file={blkFile}
          onChange={onBlkChange}
          accept=".dat"
        />
        <FileInput
          label="Undo Data"
          hint="rev*.dat"
          file={revFile}
          onChange={onRevChange}
          accept=".dat"
        />
        <FileInput
          label="XOR Key"
          hint="xor.dat"
          file={xorFile}
          onChange={onXorChange}
          accept=".dat"
        />
      </div>

      {error && (
        <div className="mb-4 p-4 rounded-lg bg-btc-red/10 border border-btc-red/20 text-btc-red text-sm">
          {error}
        </div>
      )}

      <button
        onClick={onAnalyze}
        disabled={!allSelected || loading}
        className={`w-full py-3.5 px-6 rounded-xl font-semibold text-base transition-all duration-300 flex items-center justify-center gap-2 ${
          allSelected && !loading
            ? 'bg-btc-orange hover:bg-btc-orange-light text-btc-darker cursor-pointer shadow-lg shadow-btc-orange/20 hover:shadow-btc-orange/30'
            : 'bg-btc-border text-btc-muted cursor-not-allowed'
        }`}
      >
        {loading ? (
          <>
            <Loader2 className="animate-spin" size={18} />
            Analyzing blocks...
          </>
        ) : (
          <>
            <Search size={18} />
            Analyze Block Data
          </>
        )}
      </button>

      {loading && (
        <div className="mt-4">
          <div className="h-1.5 bg-btc-darker rounded-full overflow-hidden">
            <div className="h-full bg-gradient-to-r from-btc-orange to-btc-orange-light rounded-full animate-pulse w-2/3" />
          </div>
          <p className="text-btc-muted text-xs mt-2 text-center">
            Parsing transactions and applying heuristics...
          </p>
        </div>
      )}
    </div>
  );
}

function Search({ size }: { size: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="11" cy="11" r="8" />
      <path d="m21 21-4.3-4.3" />
    </svg>
  );
}
