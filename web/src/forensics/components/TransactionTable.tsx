import { useState, useMemo } from 'react';
import { ArrowLeft } from 'lucide-react';
import type { TxOutput, NotableTxSummary } from '../types';
import {
  CLASSIFICATION_COLORS,
  CLASSIFICATION_LABELS,
} from '../types';
import HeuristicBadge from './HeuristicBadge';
import FilterBar from './FilterBar';
import TransactionDetail from './TransactionDetail';

interface TransactionTableProps {
  transactions: TxOutput[];
  blockHeight: number;
  notableTransactions: NotableTxSummary[];
  onBack: () => void;
}

const PAGE_SIZE = 50;

export default function TransactionTable({
  transactions,
  blockHeight,
  notableTransactions,
  onBack,
}: TransactionTableProps) {
  const [notableIdx, setNotableIdx] = useState(0);
  const [selectedTx, setSelectedTx] = useState<TxOutput | null>(null);
  const [activeHeuristics, setActiveHeuristics] = useState<string[]>([]);
  const [activeClassifications, setActiveClassifications] = useState<string[]>([]);
  const [activeHeuristicMode, setActiveHeuristicMode] = useState<'union' | 'intersection'>('union');
  const [searchQuery, setSearchQuery] = useState('');
  const [page, setPage] = useState(0);

  // Gather unique heuristics and classifications
  const { heuristics, classifications } = useMemo(() => {
    const hSet = new Set<string>();
    const cSet = new Set<string>();
    for (const tx of transactions) {
      cSet.add(tx.classification);
      for (const [hid, result] of Object.entries(tx.heuristics)) {
        if (result.detected) hSet.add(hid);
      }
    }
    return {
      heuristics: Array.from(hSet).sort(),
      classifications: Array.from(cSet).sort(),
    };
  }, [transactions]);

  // Filter transactions
  const filtered = useMemo(() => {
    return transactions.filter((tx) => {
      if (searchQuery && !tx.txid.toLowerCase().includes(searchQuery.toLowerCase())) {
        return false;
      }
      if (activeClassifications.length > 0 && !activeClassifications.includes(tx.classification)) {
        return false;
      }
      if (activeHeuristics.length > 0) {
        if (activeHeuristicMode === 'union') {
          const matches = activeHeuristics.some((hid) => {
            const h = tx.heuristics[hid];
            return !!h && h.detected;
          });
          if (!matches) return false;
        } else {
          const allMatch = activeHeuristics.every((hid) => {
            const h = tx.heuristics[hid];
            return !!h && h.detected;
          });
          if (!allMatch) return false;
        }
      }
      return true;
    });
  }, [transactions, searchQuery, activeClassifications, activeHeuristics, activeHeuristicMode]);

  const totalPages = Math.ceil(filtered.length / PAGE_SIZE);
  const pageItems = filtered.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);

  return (
    <div>
      {/* Header */}
      <div className="flex items-center gap-3 mb-4">
        <button
          onClick={onBack}
          className="p-2 rounded-lg hover:bg-btc-card text-btc-muted hover:text-btc-text transition-colors cursor-pointer"
        >
          <ArrowLeft size={20} />
        </button>
        <div>
          <h2 className="text-btc-text font-semibold text-lg">
            Block Height {blockHeight.toLocaleString()}
          </h2>
          <p className="text-btc-text text-sm">
            {transactions.length.toLocaleString()} transactions &middot;{' '}
            {filtered.length.toLocaleString()} matching filters
          </p>
        </div>
      </div>

      {/* Notable transactions slider (above filters) */}
      {notableTransactions && notableTransactions.length > 0 && (
        <div className='flex justify-center'>
        <div className="mb-4 md:w-1/2 ">
          <div className="text-btc-text font-semibold text-sm mb-2 text-center">Notable Transactions ({notableTransactions.length})</div>
          <div className="flex items-center gap-3">
            <button
              onClick={() => setNotableIdx((i) => (i - 1 + notableTransactions.length) % notableTransactions.length)}
              className="px-2 py-1 rounded-lg bg-btc-darker text-btc-muted hover:text-btc-text text-lg"
            >
              ‹
            </button>

            <div
              role="button"
              tabIndex={0}
              onClick={() => {
                const nt = notableTransactions[notableIdx];
                const tx = transactions.find((t) => t.txid === nt.txid);
                if (tx && !tx.is_coinbase) setSelectedTx(tx);
              }}
              className="flex-1 bg-btc-card border border-btc-border rounded-lg p-3 cursor-pointer hover:border-btc-orange/30"
            >
              {(() => {
                const nt = notableTransactions[notableIdx];
                return (
                  <div className="flex items-center justify-between">
                    <div className="min-w-0 flex flex-col gap-1">
                      <div className="text-btc-text font-mono text-xs truncate">{nt.txid}</div>
                      <div className="flex items-center gap-2">
                        
                        <span
                          className="px-2 py-0.5 rounded text-[10px] font-semibold"
                          style={{
                            backgroundColor: `${CLASSIFICATION_COLORS[nt.classification] ?? '#8b949e'}20`,
                            color: CLASSIFICATION_COLORS[nt.classification] ?? '#8b949e',
                            border: `1px solid ${CLASSIFICATION_COLORS[nt.classification] ?? '#8b949e'}30`,
                          }}
                        >
                          {CLASSIFICATION_LABELS[nt.classification] ?? nt.classification}
                        </span>
                      </div>
                    </div>
                  </div>
                );
              })()}
            </div>

            <button
              onClick={() => setNotableIdx((i) => (i + 1) % notableTransactions.length)}
              className="px-2 py-1 rounded-lg bg-btc-darker  text-lg text-btc-muted hover:text-btc-text"
            >
              ›
            </button>
          </div>
        </div>
        </div>
      )}

      {/* Filters */}
      <FilterBar
        heuristics={heuristics}
        classifications={classifications}
        activeHeuristics={activeHeuristics}
        activeClassifications={activeClassifications}
        activeHeuristicMode={activeHeuristicMode}
        searchQuery={searchQuery}
        onHeuristicChange={(h) => { setActiveHeuristics(h); setPage(0); }}
        onClassificationChange={(c) => { setActiveClassifications(c); setPage(0); }}
        onHeuristicModeChange={(m) => { setActiveHeuristicMode(m); setPage(0); }}
        onSearchChange={(q) => { setSearchQuery(q); setPage(0); }}
      />

      {/* Transaction list */}
      <div className="space-y-2 md:w-1/2 mx-auto">
        {pageItems.map((tx) => {
          const isCoinbase = tx.is_coinbase;
          const classColor =
            CLASSIFICATION_COLORS[tx.classification] ?? '#8b949e';
          const classLabel =
            CLASSIFICATION_LABELS[tx.classification] ?? tx.classification;
          const detected = Object.entries(tx.heuristics).filter(
            ([, r]) => r.detected
          );

          return (
            <div
              key={tx.txid}
              onClick={() => {
                if (!isCoinbase) setSelectedTx(tx);
              }}
              className={`bg-btc-card border border-btc-border rounded-xl p-4 transition-all group ${
                isCoinbase
                  ? 'opacity-75 cursor-not-allowed'
                  : 'hover:border-btc-orange/30 cursor-pointer'
              }`}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1.5"> {!isCoinbase &&
                    <span
                      className="px-2 py-0.5 rounded text-[10px] font-semibold"
                      style={{
                        backgroundColor: `${classColor}20`,
                        color: classColor,
                        border: `1px solid ${classColor}30`,
                      }}
                    >
                      {classLabel}
                    </span>}
                    {isCoinbase && (
                      <span className="px-2 py-0.5 rounded text-[10px] font-semibold border border-amber-500/40 text-amber-400 bg-amber-500/10">
                        COINBASE
                      </span>
                    )}
                    {detected.length > 0 && (
                      <span className="text-btc-text text-[10px]">
                        {detected.length} heuristic{detected.length !== 1 ? 's' : ''}
                      </span>
                    )}
                  </div>
                  <div className="text-btc-text font-mono text-xs truncate group-hover:text-btc-text transition-colors">
                    {tx.txid}
                  </div>
                </div>
              </div>

              {detected.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {detected.map(([id, result]) => (
                    <HeuristicBadge
                      key={id}
                      id={id}
                      result={result}
                      compact
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-2 mt-6">
          <button
            onClick={() => setPage(Math.max(0, page - 1))}
            disabled={page === 0}
            className="px-3 py-1.5 rounded-lg bg-btc-card border border-btc-border text-btc-muted text-sm disabled:opacity-30 hover:border-btc-orange/30 transition-colors cursor-pointer disabled:cursor-not-allowed"
          >
            Previous
          </button>
          <span className="text-btc-text text-sm">
            Page {page + 1} of {totalPages}
          </span>
          <button
            onClick={() => setPage(Math.min(totalPages - 1, page + 1))}
            disabled={page >= totalPages - 1}
            className="px-3 py-1.5 rounded-lg bg-btc-card border border-btc-border text-btc-muted text-sm disabled:opacity-30 hover:border-btc-orange/30 transition-colors cursor-pointer disabled:cursor-not-allowed"
          >
            Next
          </button>
        </div>
      )}

      {/* Transaction detail modal */}
      {selectedTx && (
        <TransactionDetail
          tx={selectedTx}
          onClose={() => setSelectedTx(null)}
        />
      )}
    </div>
  );
}
