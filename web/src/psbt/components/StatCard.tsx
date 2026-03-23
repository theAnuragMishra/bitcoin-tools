import Tip from "./Tooltip";

export default function StatCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint: string;
}) {
  return (
    <Tip text={hint}>
      <div className="bg-[#111827]/60 border border-white/5 rounded-xl p-4 w-full cursor-help">
        <div className="text-[10px] text-gray-500 uppercase tracking-wider mb-1.5">
          {label}
        </div>
        <div className="text-sm font-semibold text-gray-200 font-mono">
          {value}
        </div>
      </div>
    </Tip>
  );
}
