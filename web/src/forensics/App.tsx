import { useState } from 'react';
import type { FileOutput } from './types';
import Header from './components/Header';
import FileUpload from './components/FileUpload';
import HeroSection from './components/HeroSection';
import AnalysisView from './components/AnalysisView';

export default function App() {
  const [blkFile, setBlkFile] = useState<File | null>(null);
  const [revFile, setRevFile] = useState<File | null>(null);
  const [xorFile, setXorFile] = useState<File | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [data, setData] = useState<FileOutput | null>(null);

  const handleAnalyze = async () => {
    if (!blkFile || !revFile || !xorFile) return;

    setLoading(true);
    setError(null);

    try {
      const formData = new FormData();
      formData.append('blk', blkFile);
      formData.append('rev', revFile);
      formData.append('xor', xorFile);

      const res = await fetch('/api/analyze', {
        method: 'POST',
        body: formData,
      });

      const json = await res.json();

      if (!res.ok || !json.ok) {
        setError(
          json.error?.message ?? `Server error (${res.status})`
        );
        return;
      }

      setData(json as FileOutput);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to connect to server'
      );
    } finally {
      setLoading(false);
    }
  };

  const handleReset = () => {
    setData(null);
    setBlkFile(null);
    setRevFile(null);
    setXorFile(null);
    setError(null);
  };

  return (
    <div className="min-h-screen bg-btc-darker">
      <Header />
      <main className="max-w-7xl mx-auto px-6 py-8">
        {data ? (
          <AnalysisView data={data} onReset={handleReset} />
        ) : (
          <>
            <div className="text-center mb-10">
              <h1 className="text-5xl md:text-6xl font-extrabold tracking-tight mb-4">
                <span className="bg-gradient-to-r from-btc-orange via-btc-orange-light to-btc-yellow bg-clip-text text-transparent">
                  Sherlock
                </span>
              </h1>
              <p className="text-btc-muted text-lg max-w-2xl mx-auto">
                A powerful chain analysis engine for Bitcoin block data.
                Upload raw block files and uncover transaction patterns,
                wallet clusters, and privacy metrics.
              </p>
            </div>

            <FileUpload
              blkFile={blkFile}
              revFile={revFile}
              xorFile={xorFile}
              onBlkChange={setBlkFile}
              onRevChange={setRevFile}
              onXorChange={setXorFile}
              onAnalyze={handleAnalyze}
              loading={loading}
              error={error}
            />

            <HeroSection />
          </>
        )}
      </main>

      <footer className="border-t border-btc-border mt-16">
        <div className="max-w-7xl mx-auto px-6 py-6 flex items-center justify-center text-btc-muted text-xs">
          <span>Sherlock Chain Analysis Engine</span>
        </div>
      </footer>
    </div>
  );
}
