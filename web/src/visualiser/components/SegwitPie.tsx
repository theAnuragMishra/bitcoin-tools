export default function SegwitPie({
  witness,
  nonWitness,
  segwit,
}: {
  witness: number;
  nonWitness: number;
  segwit: boolean;
}) {
  const total_weight = segwit
    ? witness + nonWitness * 4
    : (witness + nonWitness) * 4;
  const total = Math.max(1, total_weight);
  const radius = 70;
  const circumference = 2 * Math.PI * radius;
  const witnessPct = ((segwit ? witness : witness * 4) / total) * 100;
  const witnessDash =
    ((segwit ? witness : witness * 4) / total) * circumference;
  const nonWitnessDash = circumference - witnessDash;

  return (
    <div className="pie-wrap">
      <svg width="200" height="200" viewBox="0 0 200 200" className="pie">
        <g transform="translate(100 100)">
          <circle className="pie-track" r={radius} cx="0" cy="0" />
          <circle
            className="pie-seg pie-nw"
            r={radius}
            cx="0"
            cy="0"
            strokeDasharray={`${nonWitnessDash} ${circumference}`}
            strokeDashoffset={0}
          />
          <circle
            className="pie-seg pie-w"
            r={radius}
            cx="0"
            cy="0"
            strokeDasharray={`${witnessDash} ${circumference}`}
            strokeDashoffset={-nonWitnessDash}
          />
        </g>
      </svg>
      <div className="pie-center">
        <div className="pie-value">{witnessPct.toFixed(0)}%</div>
        <div className="pie-label">witness wt.</div>
      </div>
    </div>
  );
}
