import {
  HEURISTIC_LABELS,
  HEURISTIC_COLORS,
  CLASSIFICATION_LABELS,
  CLASSIFICATION_COLORS,
} from '../types';

interface FilterBarProps {
  heuristics: string[];
  classifications: string[];
  activeHeuristics: string[]; 
  activeClassifications: string[];
  searchQuery: string;
  activeHeuristicMode: 'union' | 'intersection';
  onHeuristicChange: (h: string[]) => void;
  onClassificationChange: (c: string[]) => void;
  onHeuristicModeChange: (m: 'union' | 'intersection') => void;
  onSearchChange: (q: string) => void;
}

export default function FilterBar({
  heuristics,
  classifications,
  activeHeuristics,
  activeClassifications,
  activeHeuristicMode,
  searchQuery,
  onHeuristicChange,
  onClassificationChange,
  onHeuristicModeChange,
  onSearchChange,
}: FilterBarProps) {
  return (
    <div className="bg-btc-card border border-btc-border rounded-xl p-5 mb-4 space-y-4 md:w-1/2 mx-auto">
      
      {/* Search */}
      <div>
        <input
          type="text"
          placeholder="Search by TXID..."
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          className="w-full bg-btc-darker border border-btc-border rounded-lg px-4 py-2.5 text-sm text-btc-text placeholder-btc-muted/50 outline-none focus:border-btc-orange/50 transition-colors font-mono"
        />
      </div>

      {/* Classification filters */}
      <div>
        <div className="text-btc-text text-xs uppercase tracking-wider mb-2">
          Classification
        </div>
        <div className="flex flex-wrap gap-1.5">
          <FilterPill
            label="All"
            active={activeClassifications.length === 0}
            onClick={() => onClassificationChange([])}
            color="#8b949e"
          />
          {classifications.map((c) => (
            <FilterPill
              key={c}
              label={CLASSIFICATION_LABELS[c] ?? c}
              active={activeClassifications.includes(c)}
              onClick={() => {
                const next = activeClassifications.includes(c)
                  ? activeClassifications.filter((x) => x !== c)
                  : [...activeClassifications, c];
                onClassificationChange(next);
              }}
              color={CLASSIFICATION_COLORS[c] ?? '#8b949e'}
            />
          ))}
        </div>
      </div>

      {/* Heuristic filters */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <div className="text-btc-muted text-xs uppercase tracking-wider">Heuristic</div>
          <div className="flex gap-1">
            <FilterPill
              label="Any"
              active={activeHeuristicMode === 'union'}
              onClick={() => onHeuristicModeChange('union')}
              color="#8b949e"
            />
            <FilterPill
              label="All"
              active={activeHeuristicMode === 'intersection'}
              onClick={() => onHeuristicModeChange('intersection')}
              color="#8b949e"
            />
          </div>
        </div>
        <div className="flex flex-wrap gap-1.5">
          <FilterPill
            label="All"
            active={activeHeuristics.length === 0}
            onClick={() => onHeuristicChange([])}
            color="#8b949e"
          />
          {heuristics.map((h) => (
            <FilterPill
              key={h}
              label={HEURISTIC_LABELS[h] ?? h}
              active={activeHeuristics.includes(h)}
              onClick={() => {
                const next = activeHeuristics.includes(h)
                  ? activeHeuristics.filter((x) => x !== h)
                  : [...activeHeuristics, h];
                onHeuristicChange(next);
              }}
              color={HEURISTIC_COLORS[h] ?? '#8b949e'}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function FilterPill({
  label,
  active,
  onClick,
  color,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  color: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer border ${
        active
          ? 'text-btc-darker'
          : 'bg-btc-darker text-btc-text border-btc-border hover:border-btc-muted/50'
      }`}
      style={
        active
          ? { backgroundColor: color, borderColor: color }
          : undefined
      }
    >
      {label}
    </button>
  );
}
