export default function Header() {
  return (
    <header className="border-b border-btc-border bg-btc-dark/50 backdrop-blur-md sticky top-0 z-40">
      <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-btc-orange to-btc-orange-light flex items-center justify-center">
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="#010409"
              strokeWidth="2.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.3-4.3" />
            </svg>
          </div>
          <div>
            <h1 className="text-btc-text font-bold text-lg leading-tight tracking-tight">
              Sherlock
            </h1>
            <p className="text-btc-muted text-[10px] tracking-widest uppercase">
              Chain Analysis
            </p>
          </div>
        </div>
        
      </div>
    </header>
  );
}
