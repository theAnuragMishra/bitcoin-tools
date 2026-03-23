import {
  HEURISTIC_LABELS,
  HEURISTIC_COLORS,
  type HeuristicResult,
} from '../types';

interface HeuristicBadgeProps {
  id: string;
  result: HeuristicResult;
  compact?: boolean;
}

export default function HeuristicBadge({
  id,
  result,
  compact,
}: HeuristicBadgeProps) {
  if (!result.detected) return null;

  const color = HEURISTIC_COLORS[id] ?? '#8b949e';
  const label = HEURISTIC_LABELS[id] ?? id;

  if (compact) {
    return (
      <span
        className="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-medium"
        style={{
          backgroundColor: `${color}15`,
          color: color,
          border: `1px solid ${color}30`,
        }}
        title={`${label}${result.confidence ? ` (${result.confidence})` : ''}`}
      >
        {id}
      </span>
    );
  }

  return (
    <div
      className="rounded-lg p-3 border text-sm"
      style={{
        backgroundColor: `${color}08`,
        borderColor: `${color}25`,
      }}
    >
      <div className="flex items-center justify-between mb-1">
        <span
          className="t font-semibold"
          style={{ color }}
        >
          {label}
        </span>
        {result.confidence && (
          <span
            className="px-1.5 py-0.5 rounded font-medium"
            style={{
              backgroundColor: `${color}20`,
              color: color,
            }}
          >
            {result.confidence}
          </span>
        )}
      </div>
      {result.method && (
        <div className="text-btc-muted">
          Method: <span className="text-btc-text">{result.method}</span>
        </div>
      )}
      {result.details && (
        <div className="text-btc-muted mt-0.5">{result.details}</div>
      )}
      {result.likely_change_index !== undefined && (
        <div className="text-btc-muted mt-0.5">
          Change output index:{' '}
          <span className="text-btc-accent font-mono">
            {result.likely_change_index}
          </span>
        </div>
      )}
      {result.protocol && (
        <div className="text-btc-muted mt-0.5">
          Protocol: <span className="text-btc-text">{result.protocol}</span>
        </div>
      )}
      {result.fan_in_ratio !== undefined && (
        <div className="text-btc-muted mt-0.5">
          Fan-in ratio:{' '}
          <span className="text-btc-text font-mono">
            {result.fan_in_ratio.toFixed(1)}
          </span>
        </div>
      )}
      {result.equal_output_count !== undefined && (
        <div className="text-btc-muted mt-0.5">
          Equal outputs:{' '}
          <span className="text-btc-text font-mono">
            {result.equal_output_count}
          </span>
        </div>
      )}
      {result.peel_ratio !== undefined && (
        <div className="text-btc-muted mt-0.5">
          Peel ratio:{' '}
          <span className="text-btc-text font-mono">
            {result.peel_ratio.toFixed(3)}
          </span>
        </div>
      )}
      {result.privacy_score !== undefined && (
        <div className="text-btc-muted mt-0.5">
          Privacy score:{' '}
          <span
            className="font-mono font-medium"
            style={{
              color:
                result.privacy_score > 60
                  ? '#3fb950'
                  : result.privacy_score > 30
                  ? '#d29922'
                  : '#f85149',
            }}
          >
            {result.privacy_score}/100
          </span>
        </div>
      )}
    </div>
  );
}
