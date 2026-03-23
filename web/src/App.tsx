import { Link, Route, Routes } from "react-router";
import ForensicsApp from "./forensics/App";
import PsbtApp from "./psbt/App";
import VisualiserApp from "./visualiser/App";
import "./index.css";

function HomePage() {
    const routes = [
        {
            path: "/forensics",
            title: "Forensics",
            description:
                "Upload block files and inspect transaction patterns, clustering, and heuristics.",
            accent: "from-orange-500 to-amber-400",
        },
        {
            path: "/psbt",
            title: "PSBT",
            description:
                "Build and inspect partially signed transactions from JSON fixtures.",
            accent: "from-cyan-500 to-sky-400",
        },
        {
            path: "/visualiser",
            title: "Visualiser",
            description:
                "Explore transaction and block flow with the visual analysis tools.",
            accent: "from-emerald-500 to-lime-400",
        },
    ];

    return (
        <div className="min-h-screen bg-[#080b12] text-slate-100">
            <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col justify-center px-6 py-16">
                <div className="max-w-3xl space-y-8">
                    <div className="inline-flex items-center rounded-full border border-white/10 bg-white/5 px-4 py-2 text-xs uppercase tracking-[0.28em] text-slate-300">
                        Bitcoin tools workspace
                    </div>
                    <div className="space-y-4">
                        <h1 className="text-5xl font-semibold tracking-tight text-white md:text-6xl">
                            Choose a Bitcoin analysis tool.
                        </h1>
                        <p className="max-w-2xl text-base leading-7 text-slate-300 md:text-lg">
                            This workspace serves three dedicated UIs for forensics, PSBT
                            construction, and transaction visualisation. Open the route that
                            matches the task you want to work on.
                        </p>
                    </div>

                    <div className="grid gap-4 md:grid-cols-3">
                        {routes.map((route) => (
                            <Link
                                key={route.path}
                                to={route.path}
                                className="group rounded-2xl border border-white/10 bg-white/5 p-5 transition-transform duration-200 hover:-translate-y-1 hover:border-white/20 hover:bg-white/10"
                            >
                                <div
                                    className={`mb-4 h-12 w-12 rounded-xl bg-linear-to-br ${route.accent} shadow-lg shadow-black/30`}
                                />
                                <h2 className="text-xl font-semibold text-white">{route.title}</h2>
                                <p className="mt-2 text-sm leading-6 text-slate-300">
                                    {route.description}
                                </p>
                                <div className="mt-5 text-sm font-medium text-slate-200 transition-colors group-hover:text-white">
                                    Open {route.title}
                                </div>
                            </Link>
                        ))}
                    </div>
                </div>
            </main>
        </div>
    );
}

export default function App() {
    return (
        <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/forensics" element={<ForensicsApp />} />
            <Route path="/psbt" element={<PsbtApp />} />
            <Route path="/visualiser" element={<VisualiserApp />} />
        </Routes>
    );
}