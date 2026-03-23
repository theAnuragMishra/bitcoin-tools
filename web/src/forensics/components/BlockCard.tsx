import { ChevronDown, ChevronRight, Layers } from 'lucide-react';
import { useState } from 'react';
import type { BlockOutput } from '../types';
import {
  SCRIPT_TYPE_COLORS,
} from '../types';
import {
  PieChart,
  Pie,
  Cell,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

interface BlockCardProps {
  block: BlockOutput;
  index: number;
  isSelected: boolean;
  onSelect: () => void;
}

export default function BlockCard({
  block,
  index,
  isSelected,
  onSelect,
}: BlockCardProps) {
  const [expanded, setExpanded] = useState(false);

  const scriptData = Object.entries(
    block.analysis_summary.script_type_distribution
  )
    .map(([name, value]) => ({
      name: name.toUpperCase(),
      value,
      fill: SCRIPT_TYPE_COLORS[name] ?? '#484f58',
    }))
    .sort((a, b) => b.value - a.value);

  const fee = block.analysis_summary.fee_rate_stats;
  const flaggedPct =
    block.tx_count > 0
      ? (
          (block.analysis_summary.flagged_transactions / block.tx_count) *
          100
        ).toFixed(1)
      : '0';

  return (
    <div
      className={`bg-btc-card border rounded-xl transition-all duration-300 ${
        isSelected
          ? 'border-btc-orange/50 shadow-lg shadow-btc-orange/10'
          : 'border-btc-border hover:border-btc-border'
      }`}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between p-5 cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-3 min-w-0">
          <div className="w-9 h-9 rounded-lg bg-btc-orange/10 flex items-center justify-center shrink-0">
            <Layers className="text-btc-orange" size={18} />
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-btc-text font-semibold text-sm">
                Block #{index + 1}
              </span>
              <span className="text-btc-muted text-xs">
                Height {block.block_height.toLocaleString()}
              </span>
            </div>
            <div className="text-btc-muted font-mono text-xs truncate">
              {block.block_hash}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4 shrink-0">
          <div className="hidden md:flex items-center gap-4 text-xs">
            <span className="text-btc-accent">
              {block.tx_count.toLocaleString()} txs
            </span>
            <span className="text-btc-red">
              {block.analysis_summary.flagged_transactions.toLocaleString()}{' '}
              flagged ({flaggedPct}%)
            </span>
            <span className="text-btc-green">
              {fee.median_sat_vb.toFixed(1)} sat/vB median
            </span>
          </div>
          {expanded ? (
            <ChevronDown className="text-btc-muted" size={18} />
          ) : (
            <ChevronRight className="text-btc-muted" size={18} />
          )}
        </div>
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div className="border-t border-btc-border p-5">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-5">
            <MiniStat label="Min Fee" value={`${fee.min_sat_vb.toFixed(1)} sat/vB`} />
            <MiniStat label="Median Fee" value={`${fee.median_sat_vb.toFixed(1)} sat/vB`} />
            <MiniStat label="Mean Fee" value={`${fee.mean_sat_vb.toFixed(1)} sat/vB`} />
            <MiniStat label="Max Fee" value={`${fee.max_sat_vb.toFixed(1)} sat/vB`} />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
            {/* Script distribution mini chart */}
            <div>
              <h4 className="text-btc-muted text-xs uppercase tracking-wider mb-3">
                Script Types
              </h4>
              <ResponsiveContainer width="100%" height={180}>
                <PieChart>
                  <Pie
                    data={scriptData}
                    cx="50%"
                    cy="50%"
                    innerRadius={35}
                    outerRadius={65}
                    paddingAngle={2}
                    dataKey="value"
                  >
                    {scriptData.map((entry, i) => (
                      <Cell key={i} fill={entry.fill} stroke="transparent" />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      background: '#161b22',
                      border: '1px solid #30363d',
                      borderRadius: '8px',
                      fontSize: '11px',
                      color: '#e6edf3',
                    }}
                    labelStyle={{ color: '#e6edf3' }}
                    itemStyle={{ color: '#e6edf3' }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>

            {/* Script type list */}
            <div>
              <h4 className="text-btc-muted text-xs uppercase tracking-wider mb-3">
                Breakdown
              </h4>
              <div className="space-y-1.5">
                {scriptData.map((s) => (
                  <div
                    key={s.name}
                    className="flex items-center justify-between text-xs"
                  >
                    <div className="flex items-center gap-2">
                      <div
                        className="w-2.5 h-2.5 rounded-sm"
                        style={{ backgroundColor: s.fill }}
                      />
                      <span className="text-btc-muted font-mono">{s.name}</span>
                    </div>
                    <span className="text-btc-text">
                      {s.value.toLocaleString()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* View transactions button */}
          {block.transactions && block.transactions.length > 0 && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onSelect();
              }}
              className="mt-5 w-full py-2.5 rounded-lg bg-btc-orange/10 hover:bg-btc-orange/20 text-btc-orange text-sm font-medium transition-colors cursor-pointer border border-btc-orange/20"
            >
              Explore {block.transactions.length.toLocaleString()} Transactions
            </button>
          )}
          {(!block.transactions || block.transactions.length === 0) && (
            <p className="mt-4 text-btc-muted text-xs text-center italic">
              Transaction details not included for this block. 
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-btc-darker rounded-lg p-3">
      <div className="text-btc-muted text-[10px] uppercase tracking-wider">
        {label}
      </div>
      <div className="text-btc-text text-sm font-mono mt-0.5">{value}</div>
    </div>
  );
}
