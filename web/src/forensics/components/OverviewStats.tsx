import {
  CLASSIFICATION_COLORS,
  CLASSIFICATION_LABELS,
  SCRIPT_TYPE_COLORS,
  type AnalysisSummary,
} from '../types';
import {
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';

interface OverviewStatsProps {
  summary: AnalysisSummary;
  blockCount: number;
  fileName: string;
  classificationCounts?: Record<string, number>;
}

export default function OverviewStats({
  summary,
  blockCount,
  fileName,
  classificationCounts,
}: OverviewStatsProps) {
  const scriptData = Object.entries(summary.script_type_distribution)
    .map(([name, value]) => ({
      name: name.toUpperCase(),
      value,
      fill: SCRIPT_TYPE_COLORS[name] ?? '#484f58',
    }))
    .sort((a, b) => b.value - a.value);

  const classData = classificationCounts
    ? Object.entries(classificationCounts)
        .map(([name, value]) => ({
          name: CLASSIFICATION_LABELS[name] ?? name,
          value,
          fill: CLASSIFICATION_COLORS[name] ?? '#484f58',
        }))
        .sort((a, b) => b.value - a.value)
    : [];

  const fee = summary.fee_rate_stats;
  const feeData = [
    { name: 'Min', value: fee.min_sat_vb, fill: '#3fb950' },
    { name: 'Median', value: fee.median_sat_vb, fill: '#58a6ff' },
    { name: 'Mean', value: fee.mean_sat_vb, fill: '#bc8cff' },
    { name: 'Max', value: fee.max_sat_vb, fill: '#f85149' },
  ];

  const flaggedPct =
    summary.total_transactions_analyzed > 0
      ? (
          (summary.flagged_transactions /
            summary.total_transactions_analyzed) *
          100
        ).toFixed(1)
      : '0';

  return (
    <div>
      {/* Stats bar */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
        <StatCard
          label="Source File"
          value={fileName}
          color="text-btc-orange"
          mono
        />
        <StatCard
          label="Blocks"
          value={blockCount.toLocaleString()}
          color="text-btc-accent"
        />
        <StatCard
          label="Transactions"
          value={summary.total_transactions_analyzed.toLocaleString()}
          color="text-btc-green"
        />
        <StatCard
          label="Flagged"
          value={`${summary.flagged_transactions.toLocaleString()} (${flaggedPct}%)`}
          color="text-btc-red"
        />
      </div>

      {/* Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
        {/* Script type pie chart */}
        <div className="bg-btc-card border border-btc-border rounded-xl p-6">
          <h3 className="text-btc-text font-semibold mb-4">
            Script Type Distribution
          </h3>
          <ResponsiveContainer width="100%" height={260}>
            <PieChart>
              <Pie
                data={scriptData}
                cx="50%"
                cy="50%"
                innerRadius={50}
                outerRadius={90}
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
                  fontSize: '12px',
                  color: '#e6edf3',
                }}
                labelStyle={{ color: '#e6edf3' }}
                itemStyle={{ color: '#e6edf3' }}
              />
              <Legend
                wrapperStyle={{ fontSize: '11px', color: '#8b949e' }}
              />
            </PieChart>
          </ResponsiveContainer>
        </div>

        {/* Fee rate stats  */}
        <div className="bg-btc-card border border-btc-border rounded-xl p-6">
          <h3 className="text-btc-text font-semibold mb-4">
            Fee Rate Statistics (sat/vB)
          </h3>
          <div className="grid grid-cols-2 gap-4">
            <FeeStat label="Min" value={fee.min_sat_vb} color="#3fb950" reference={fee.median_sat_vb} />
            <FeeStat label="Median" value={fee.median_sat_vb} color="#58a6ff" reference={fee.median_sat_vb} />
            <FeeStat label="Mean" value={fee.mean_sat_vb} color="#bc8cff" reference={fee.median_sat_vb} />
            <FeeStat label="Max" value={fee.max_sat_vb} color="#f85149" reference={fee.median_sat_vb} />
          </div>
        </div>

        {/* Classification pie chart */}
        {classData.length > 0 && (
          <div className="bg-btc-card border border-btc-border rounded-xl p-6">
            <h3 className="text-btc-text font-semibold mb-4">
              Transaction Classifications
            </h3>
            <ResponsiveContainer width="100%" height={260}>
              <PieChart>
                <Pie
                  data={classData}
                  cx="50%"
                  cy="50%"
                  innerRadius={50}
                  outerRadius={90}
                  paddingAngle={2}
                  dataKey="value"
                >
                  {classData.map((entry, i) => (
                    <Cell key={i} fill={entry.fill} stroke="transparent" />
                  ))}
                </Pie>
                <Tooltip
                  contentStyle={{
                    background: '#161b22',
                    border: '1px solid #30363d',
                    borderRadius: '8px',
                    fontSize: '12px',
                    color: '#e6edf3',
                  }}
                  labelStyle={{ color: '#e6edf3' }}
                  itemStyle={{ color: '#e6edf3' }}
                />
                <Legend
                  wrapperStyle={{ fontSize: '11px', color: '#8b949e' }}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
        )}
      </div>

      {/* Heuristics Applied */}
      <div className="bg-btc-card border border-btc-border rounded-xl p-6 mb-8">
        <h3 className="text-btc-text font-semibold mb-4">
          Heuristics Applied
        </h3>
        <div className="flex flex-wrap gap-2">
          {summary.heuristics_applied.map((h) => (
            <span
              key={h}
              className="px-3 py-1.5 rounded-lg bg-btc-darker border border-btc-border text-btc-muted text-xs font-mono"
            >
              {h}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  color,
  mono,
}: {
  label: string;
  value: string;
  color: string;
  mono?: boolean;
}) {
  return (
    <div className="bg-btc-card border border-btc-border rounded-xl p-5">
      <div className="text-btc-muted text-xs uppercase tracking-wider mb-1.5">
        {label}
      </div>
      <div
        className={`${color} font-semibold text-lg truncate ${mono ? 'font-mono text-sm' : ''}`}
      >
        {value}
      </div>
    </div>
  );
}

function FeeStat({
  label,
  value,
  color,
  reference,
}: {
  label: string;
  value: number;
  color?: string;
  reference?: number;
}) {
  const isFiniteValue = Number.isFinite(value);
  const formatted = isFiniteValue ? value.toFixed(1) : '-';
  const ratio = reference && reference > 0 && isFiniteValue ? value / reference : null;
  const ratioText = ratio == null ? null : `${ratio >= 1 ? ratio.toFixed(1) : ratio.toFixed(2)}× median`;

  return (
    <div className="bg-btc-darker rounded-lg p-4 border border-btc-border">
      <div className="text-btc-muted text-[10px] uppercase tracking-wider">{label}</div>
      <div className="flex items-baseline gap-2 mt-2">
        <div style={{ width: 10, height: 10, background: color }} className="rounded-sm shrink-0" />
        <div className="text-btc-text font-mono text-lg font-semibold">
          {formatted} <span className="text-btc-muted text-sm">sat/vB</span>
        </div>
      </div>
      {ratioText && <div className="text-btc-muted text-[11px] mt-1">{ratioText}</div>}
    </div>
  );
}
