import { Search, Shield, Eye, Zap, GitBranch, Database } from 'lucide-react';

const features = [
  {
    icon: Search,
    title: 'Common Input Ownership',
    description:
      'Identifies transactions where multiple inputs are spent together, suggesting they belong to the same wallet entity.',
    color: 'text-btc-orange',
  },
  {
    icon: Eye,
    title: 'Change Detection',
    description:
      'Analyzes output patterns to identify which output is likely change returning to the sender using script type matching and round number analysis.',
    color: 'text-btc-accent',
  },
  {
    icon: Shield,
    title: 'CoinJoin Detection',
    description:
      'Detects privacy-enhancing CoinJoin transactions with symmetric equal-value outputs designed to obscure the transaction graph.',
    color: 'text-btc-purple',
  },
  {
    icon: Database,
    title: 'Consolidation Analysis',
    description:
      'Identifies wallet maintenance transactions that combine many small UTXOs into fewer outputs, reducing the UTXO set size.',
    color: 'text-btc-cyan',
  },
  {
    icon: GitBranch,
    title: 'Peeling Chain Detection',
    description:
      'Traces peeling chain patterns where a large amount is repeatedly split into small payments and change outputs.',
    color: 'text-btc-pink',
  },
  {
    icon: Zap,
    title: 'OP_RETURN Analysis',
    description:
      'Detects and classifies embedded data in OP_RETURN outputs, identifying protocols like Omni and OpenTimestamps.',
    color: 'text-btc-muted',
  },
];

export default function HeroSection() {
  return (
    <div className="mt-12 mb-8">
      <div className="text-center mb-16">
        <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-btc-orange/10 border border-btc-orange/20 mb-8">
          <span className="w-2 h-2 rounded-full bg-btc-orange animate-pulse" />
          <span className="text-btc-orange text-sm font-medium">
            Bitcoin Chain Analysis Engine
          </span>
        </div>

        <h2 className="text-4xl md:text-5xl font-bold text-btc-text mb-6 tracking-tight">
          What is{' '}
          <span className="bg-gradient-to-r from-btc-orange to-btc-orange-light bg-clip-text text-transparent">
            Chain Analysis
          </span>
          ?
        </h2>

        <p className="text-btc-muted text-lg max-w-3xl mx-auto leading-relaxed">
          Chain analysis applies heuristic techniques to Bitcoin's public
          blockchain to cluster addresses, trace fund flows, and infer
          transaction behavior. While Bitcoin is pseudonymous, patterns in
          transaction structure reveal information about sender and receiver
          relationships, wallet types, and privacy practices.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
        {features.map((feature) => {
          const Icon = feature.icon;
          return (
            <div
              key={feature.title}
              className="group relative bg-btc-card border border-btc-border rounded-xl p-6 hover:border-btc-orange/30 transition-all duration-300 hover:shadow-lg hover:shadow-btc-orange/5"
            >
              <div
                className={`w-10 h-10 rounded-lg bg-btc-darker flex items-center justify-center mb-4 ${feature.color}`}
              >
                <Icon size={20} />
              </div>
              <h3 className="text-btc-text font-semibold text-base mb-2">
                {feature.title}
              </h3>
              <p className="text-btc-muted text-sm leading-relaxed">
                {feature.description}
              </p>
            </div>
          );
        })}
      </div>

      <div className="mt-16 bg-btc-card border border-btc-border rounded-xl p-8">
        <h3 className="text-xl font-semibold text-btc-text mb-4">
          How Heuristics Work
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          <div>
            <div className="text-btc-orange font-mono text-sm font-semibold mb-2">
              01 / INPUT
            </div>
            <p className="text-btc-muted text-sm leading-relaxed">
              Upload raw Bitcoin block data files (blk*.dat, rev*.dat, xor.dat)
              directly from Bitcoin Core's data directory.
            </p>
          </div>
          <div>
            <div className="text-btc-accent font-mono text-sm font-semibold mb-2">
              02 / ANALYSIS
            </div>
            <p className="text-btc-muted text-sm leading-relaxed">
              The engine parses raw transactions, reconstructs prevouts, and
              applies 9 different heuristic algorithms to each transaction.
            </p>
          </div>
          <div>
            <div className="text-btc-green font-mono text-sm font-semibold mb-2">
              03 / INSIGHT
            </div>
            <p className="text-btc-muted text-sm leading-relaxed">
              Explore classified transactions, flagged patterns, fee rate
              distributions, and script type breakdowns across entire blocks.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
