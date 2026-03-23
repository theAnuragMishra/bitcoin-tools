import { useState, useMemo } from 'react';
import type { FileOutput } from '../types';
import OverviewStats from './OverviewStats';
import BlockCard from './BlockCard';
import TransactionTable from './TransactionTable';

interface AnalysisViewProps {
  data: FileOutput;
  onReset: () => void;
}

export default function AnalysisView({ data, onReset }: AnalysisViewProps) {
  const [selectedBlockIdx, setSelectedBlockIdx] = useState<number | null>(null);
  // Track which clusters are collapsed. Initialize collapsed by default so
  // clusters start folded when the view renders.
  const [collapsedClusters, setCollapsedClusters] = useState<Set<number>>(() => {
    const count = data.address_clusters?.length ?? 0;
    return new Set(Array.from({ length: count }, (_, i) => i));
  });

  // Build classification counts from the first block's transactions
  const classificationCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const block of data.blocks) {
      if (block.transactions && block.transactions.length > 0) {
        for (const tx of block.transactions) {
          counts[tx.classification] = (counts[tx.classification] ?? 0) + 1;
        }
      }
    }
    return Object.keys(counts).length > 0 ? counts : undefined;
  }, [data]);

  const clusterCount = data.address_clusters?.length ?? 0;

  // If a block is selected for tx exploration
  if (selectedBlockIdx !== null) {
    const block = data.blocks[selectedBlockIdx];
    return (
      <TransactionTable
        transactions={block.transactions}
        blockHeight={block.block_height}
        notableTransactions={block.notable_transactions ?? []}
        onBack={() => setSelectedBlockIdx(null)}
      />
    );
  }

  return (
    <div>
      {/* Header */}
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-btc-text">
            Analysis Results
          </h1>
          <p className="text-btc-muted text-sm mt-1">
            Heuristic analysis of{' '}
            <span className="font-mono text-btc-orange">{data.file}</span>
          </p>
        </div>
        <button
          onClick={onReset}
          className="px-4 py-2 rounded-lg bg-btc-card border border-btc-border text-btc-muted hover:text-btc-text hover:border-btc-orange/30 transition-colors text-sm cursor-pointer"
        >
          New Analysis
        </button>
      </div>

      {/* Overview stats and charts */}
      <OverviewStats
        summary={data.analysis_summary}
        blockCount={data.block_count}
        fileName={data.file}
        classificationCounts={classificationCounts}
      />

      {/* Address clustering (CIOH-based, coinjoin-excluded) */}
      <div className="bg-btc-card border border-btc-border rounded-xl p-6 mb-8">
        <div className="flex items-center justify-between gap-3 mb-4">
          <h2 className="text-btc-text font-semibold text-lg">Address Clusters</h2>
          <span className="text-xs text-btc-muted">
            {clusterCount} cluster{clusterCount === 1 ? '' : 's'} (size 3+)
          </span>
        </div>

        {clusterCount === 0 ? (
          <p className="text-btc-muted text-sm">
            No qualifying address clusters found for this analysis window.
          </p>
        ) : (
          <div className="space-y-4 max-h-[26rem] overflow-y-auto pr-1">
            {data.address_clusters.map((cluster, idx) => {
              const isCollapsed = collapsedClusters.has(idx);
              // calculate a reasonable max-height for the expanded body to
              // allow smooth transitions; larger clusters will be capped.
              const expandedMaxHeight = Math.min(1000, cluster.length * 40 + 24);
              return (
                <div
                  key={`cluster-${idx}`}
                  className="rounded-lg border border-btc-border bg-btc-darker p-4"
                >
                  <div className="flex items-center justify-between gap-3 mb-3">
                    <div className="flex items-center gap-3">
                      <button
                        onClick={() => {
                          setCollapsedClusters((prev) => {
                            const next = new Set(prev);
                            if (next.has(idx)) next.delete(idx);
                            else next.add(idx);
                            return next;
                          });
                        }}
                        aria-expanded={!isCollapsed}
                        aria-controls={`cluster-body-${idx}`}
                        title={isCollapsed ? 'Expand cluster' : 'Collapse cluster'}
                        className="w-7 h-7 inline-flex items-center justify-center rounded-md bg-btc-card border border-btc-border text-btc-text text-sm"
                      >
                        {isCollapsed ? (
                          <svg
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            className="w-4 h-4"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth={2}
                          >
                            <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
                          </svg>
                        ) : (
                          <svg
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            className="w-4 h-4"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth={2}
                          >
                            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                          </svg>
                        )}
                      </button>
                      <span className="text-sm font-semibold text-btc-orange">
                        Cluster {idx + 1}
                      </span>
                    </div>

                    <span className="text-xs text-btc-muted">
                      {cluster.length} addresses
                    </span>
                  </div>

                  {/* Collapsible body */}
                  {/* Collapsible body with smooth max-height + opacity transition */}
                  <div
                    id={`cluster-body-${idx}`}
                    style={{
                      maxHeight: isCollapsed ? 0 : expandedMaxHeight,
                      overflow: 'hidden',
                      transition: 'max-height 240ms ease, opacity 200ms ease',
                      opacity: isCollapsed ? 0 : 1,
                    }}
                  >
                    <div className="flex flex-wrap gap-2 py-1">
                      {cluster.map((fp) => (
                        <span
                          key={fp}
                          className="px-2.5 py-1 rounded-md border border-btc-border text-xs text-btc-text font-mono bg-btc-card"
                          title={fp}
                        >
                          {shortFingerprint(fp)}
                        </span>
                      ))}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Block list */}
      <div className="mb-4">
        <h2 className="text-btc-text font-semibold text-lg mb-4">
          Blocks ({data.block_count})
        </h2>
        <div className="space-y-3">
          {data.blocks.map((block, i) => (
            <BlockCard
              key={block.block_hash}
              block={block}
              index={i}
              isSelected={false}
              onSelect={() => setSelectedBlockIdx(i)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function shortFingerprint(fp: string): string {
  if (fp.length <= 20) return fp;
  return `${fp.slice(0, 10)}...${fp.slice(-8)}`;
}
