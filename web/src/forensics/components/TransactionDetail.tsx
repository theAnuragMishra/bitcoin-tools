import { X } from 'lucide-react';
import type { TxOutput } from '../types';
import {
  CLASSIFICATION_COLORS,
  CLASSIFICATION_LABELS,
} from '../types';
import HeuristicBadge from './HeuristicBadge';

interface TransactionDetailProps {
  tx: TxOutput;
  onClose: () => void;
}

export default function TransactionDetail({
  tx,
  onClose,
}: TransactionDetailProps) {
  const detectedHeuristics = Object.entries(tx.heuristics).filter(
    ([, r]) => r.detected
  );
  const undetectedHeuristics = Object.entries(tx.heuristics).filter(
    ([, r]) => !r.detected
  );
  const classColor =
    CLASSIFICATION_COLORS[tx.classification] ?? '#8b949e';
  const classLabel =
    CLASSIFICATION_LABELS[tx.classification] ?? tx.classification;
  const totalIn = tx.total_input_value;
  const totalOut = tx.total_output_value;
  const totalFee = tx.total_fee;
  const feeRateSatPerVb =
    totalFee !== undefined && tx.vbytes > 0 ? totalFee / tx.vbytes : undefined;
  

  const formatSats = (value?: number) =>
    value === undefined ? 'N/A' : `${value.toLocaleString()} sats`;
  const formatBtc = (value?: number) =>
    value === undefined ? 'N/A' : `${(value / 100_000_000).toFixed(8)} BTC`;

  const outputEntries = tx.outputs ?? [];
  const inputEntries = tx.inputs ?? [];

  return (
    <div className="fixed inset-0 z-50 bg-btc-darker/95 backdrop-blur-sm">
      <div className="bg-btc-darker w-full h-full overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-btc-border bg-btc-card/80">
          <div className="min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="text-btc-text font-semibold text-lg">
                Transaction Details
              </h3>
              <span
                className="px-2.5 py-0.5 rounded text-sm font-medium"
                style={{
                  backgroundColor: `${classColor}20`,
                  color: classColor,
                  border: `1px solid ${classColor}30`,
                }}
              >
                {classLabel}
              </span>
            </div>
            <div className="text-btc-text font-mono text-sm truncate">
              {tx.txid}
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-btc-darker text-btc-text hover:text-btc-text transition-colors cursor-pointer"
          >
            <X size={18} />
          </button>
        </div>

        {/* Body */}
        <div className="overflow-y-auto p-6 space-y-6 max-w-7xl mx-auto w-full">
          {/* Flow visualizer */}
          <div className="bg-btc-card border border-btc-border rounded-xl overflow-hidden">
            <div className="px-5 py-4 border-b border-btc-border">
              <h4 className="text-base font-semibold text-btc-text">Transaction Flow</h4>
              
            </div>
            <div className="p-5">
              <div className="grid grid-cols-1 lg:grid-cols-[1fr_auto_1fr] gap-4 items-center">
                <div className="space-y-2">
                  <div className="text-xs text-btc-muted uppercase tracking-wider font-medium">
                    Inputs
                  </div>
                  <div className="bg-emerald-500/5 border border-emerald-500/20 rounded-xl p-3">
                    <div className="text-xs text-emerald-300/70 mb-2">
                      Total in: {formatBtc(totalIn)} ({formatSats(totalIn)})
                    </div>
                    <div className="space-y-1.5 max-h-52 overflow-y-auto pr-1">
                      {inputEntries.length > 0 ? (
                        inputEntries.map((input) => (
                          <div
                            key={`in-${input.index}`}
                            className="rounded-lg border border-emerald-500/20 bg-btc-darker px-3 py-2"
                          >
                            <div className="text-xs text-btc-muted">Input #{input.index}</div>
                            {input.is_coinbase ? (
                              <div className="text-sm font-semibold text-amber-300">Coinbase input</div>
                            ) : (
                              <>
                                <div className="text-sm font-mono text-emerald-300">
                                  {formatBtc(input.value_sats)}
                                </div>
                                <div className="text-xs text-btc-muted">
                                  {formatSats(input.value_sats)} · {input.script_type ?? 'unknown'}
                                </div>
                              </>
                            )}
                          </div>
                        ))
                      ) : (
                        <div className="text-sm text-btc-muted">No input details available</div>
                      )}
                    </div>
                  </div>
                  
                </div>

                <div className="flex flex-col items-center justify-center py-2 px-1 min-w-[9rem]">
                  <div className="w-px h-6 bg-gradient-to-b from-transparent via-btc-orange to-transparent" />
                  <div className="border border-btc-orange/50 rounded-lg p-3 text-center bg-btc-darker w-full">
                    <div className="text-xl text-btc-orange leading-none">₿</div>
                    <div className="text-[11px] text-btc-muted mt-1 font-semibold uppercase tracking-wide">
                      Network Fee
                    </div>
                    <div className="mt-2 bg-btc-orange/10 border border-btc-orange/40 rounded-md px-2 py-1.5 text-sm font-semibold text-btc-orange">
                      {formatSats(totalFee)}
                    </div>
                    {feeRateSatPerVb !== undefined && (
                  <div className="text-btc-muted text-xs mt-1">
                    {feeRateSatPerVb.toFixed(1)} sat/vB
                  </div>
                )}
                  </div>
                  <div className="w-px h-6 bg-gradient-to-b from-transparent via-btc-orange to-transparent" />
                </div>

                <div className="space-y-2">
                  <div className="text-xs text-btc-muted uppercase tracking-wider font-medium">
                    Outputs (Right)
                  </div>
                  <div className="bg-blue-500/5 border border-blue-500/20 rounded-xl p-3">
                    <div className="text-xs text-blue-300/70 mb-2">
                      Total out: {formatBtc(totalOut)} ({formatSats(totalOut)})
                    </div>
                    <div className="space-y-1.5 max-h-52 overflow-y-auto pr-1">
                      {outputEntries.length > 0 ? (
                        outputEntries.map((out) => (
                          <div
                            key={`out-${out.index}`}
                            className="rounded-lg border border-blue-500/20 bg-btc-darker px-3 py-2"
                          >
                            <div className="text-xs text-btc-muted">Output #{out.index}</div>
                            <div className="text-sm font-mono text-blue-300">
                              {formatBtc(out.value_sats)}
                            </div>
                            <div className="text-xs text-btc-muted">{formatSats(out.value_sats)} · {out.script_type}</div>
                          </div>
                        ))
                      ) : (
                        <div className="text-sm text-btc-muted">No output details available</div>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Required tx-level details */}
          <div className="bg-btc-card border border-btc-border rounded-xl p-4">
            <h4 className="text-btc-text text-base font-semibold mb-3">
              Transaction-Level Summary
            </h4>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-6 gap-3 text-sm">
              <div className="rounded-lg border border-btc-border bg-btc-darker p-3">
                <div className="text-btc-muted mb-1">Input Count</div>
                <div className="text-btc-text font-semibold">{tx.input_count ?? 'N/A'}</div>
              </div>
              <div className="rounded-lg border border-btc-border bg-btc-darker p-3">
                <div className="text-btc-muted mb-1">Output Count</div>
                <div className="text-btc-text font-semibold">{tx.output_count ?? 'N/A'}</div>
              </div>
              <div className="rounded-lg border border-btc-border bg-btc-darker p-3">
                <div className="text-btc-muted mb-1">Total Input Value</div>
                <div className="text-btc-text font-semibold">{formatSats(tx.total_input_value)}</div>
                <div className="text-btc-muted">{formatBtc(tx.total_input_value)}</div>
              </div>
              <div className="rounded-lg border border-btc-border bg-btc-darker p-3">
                <div className="text-btc-muted mb-1">Total Output Value</div>
                <div className="text-btc-text font-semibold">{formatSats(tx.total_output_value)}</div>
                <div className="text-btc-muted">{formatBtc(tx.total_output_value)}</div>
              </div>
              <div className="rounded-lg border border-btc-border bg-btc-darker p-3">
                <div className="text-btc-muted mb-1">Size (vbytes)</div>
                <div className="text-btc-text font-semibold">{tx.vbytes ?? 'N/A'} vB</div>
               
              </div>
              <div className="rounded-lg border border-btc-border bg-btc-darker p-3">
                <div className="text-btc-muted mb-1">Total Fee</div>
                <div className="text-btc-text font-semibold">{formatSats(tx.total_fee)}</div>
                <div className="text-btc-muted">{formatBtc(tx.total_fee)}</div>
                {feeRateSatPerVb !== undefined && (
                  <div className="text-btc-muted text-xs mt-1">
                    @ {feeRateSatPerVb.toFixed(1)} sat/vB
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Detected heuristics */}
          {detectedHeuristics.length > 0 && (
            <div className="bg-btc-card border border-btc-border rounded-xl p-4">
              <h4 className="text-btc-text text-sm font-semibold mb-3">
                Detected Heuristics ({detectedHeuristics.length})
              </h4>
              <div className="flex flex-col gap-2">
                {detectedHeuristics.map(([id, result]) => (
                  <HeuristicBadge key={id} id={id} result={result} />
                ))}
              </div>
            </div>
          )}

          {/* Undetected heuristics */}
          {undetectedHeuristics.length > 0 && (
            <div className="bg-btc-card border border-btc-border rounded-xl p-4">
              <h4 className="text-btc-text text-sm font-semibold mb-2 uppercase tracking-wider">
                Not Detected ({undetectedHeuristics.length})
              </h4>
              <div className="flex flex-wrap gap-1.5">
                {undetectedHeuristics.map(([id]) => (
                  <span
                    key={id}
                    className="px-2.5 py-1 rounded bg-btc-darker border border-btc-border text-btc-text text-xs font-mono"
                  >
                    {id}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
