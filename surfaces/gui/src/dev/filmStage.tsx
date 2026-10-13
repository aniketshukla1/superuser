// The launch film's review stage (film.html, development only — never built). Plays the
// film with a pretend runtime so it can be watched, replayed and judged without restarting
// the app: the full cut or the short one, a runtime that answers, is slow, or never does.
// URL parameters set the same things (`?cut=short&runtime=slow&leave=1`), for scripted
// captures.

import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/inter/400.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/800.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "../tailwind.css";
import "../styles.css";
import { LaunchFilm } from "../components/LaunchFilm/LaunchFilm";
import type { BootStep } from "../components/LaunchFilm/intro";

type Runtime = "ready" | "slow" | "failed";
const q = new URLSearchParams(location.search);

function Take({ cut, runtime, leave }: { cut: "full" | "short"; runtime: Runtime; leave: boolean }) {
  const [t, setT] = useState(0);
  const [skipped, setSkipped] = useState(false);
  const [ended, setEnded] = useState(false);
  const [gone, setGone] = useState(false);
  useEffect(() => {
    const born = performance.now();
    const id = window.setInterval(() => setT(performance.now() - born), 100);
    return () => clearInterval(id);
  }, []);
  const up = (ms: number) => runtime === "ready" && t > ms;
  const steps: BootStep[] = [
    { label: "Local runtime", state: up(1200) ? true : runtime === "failed" && t > 3000 ? "failed" : false },
    { label: up(1600) ? "19 assistants" : "Assistants", state: up(1600) },
    { label: "New chat", state: up(2000) },
    { label: "Live connection", state: up(2400) },
  ];
  const ready = steps.every((s) => s.state === true);
  if (gone) return <p className="film-stage-note">The app would be on screen now.</p>;
  return (
    <div className="boot-splash film-stage-frame">
      <LaunchFilm
        steps={steps}
        failed={runtime === "failed" && t > 3000}
        onRetry={() => location.reload()}
        intro={cut}
        skipped={skipped}
        onSkip={() => setSkipped(true)}
        onReelEnd={() => setEnded(true)}
        leaving={leave && ready && (ended || skipped)}
        onLeft={() => setGone(true)}
      />
    </div>
  );
}

function Stage() {
  const [cut, setCut] = useState<"full" | "short">(q.get("cut") === "short" ? "short" : "full");
  const [runtime, setRuntime] = useState<Runtime>((q.get("runtime") as Runtime) || "ready");
  const [leave, setLeave] = useState(q.get("leave") === "1");
  const [take, setTake] = useState(0);
  const bar = q.get("bare") !== "1";
  return (
    <>
      <Take key={`${take}-${cut}-${runtime}-${leave}`} cut={cut} runtime={runtime} leave={leave} />
      {bar && (
        <div className="film-stage-bar">
          <button onClick={() => setTake((n) => n + 1)}>replay</button>
          {(["full", "short"] as const).map((c) => (
            <button key={c} className={cut === c ? "is-on" : ""} onClick={() => setCut(c)}>{c}</button>
          ))}
          {(["ready", "slow", "failed"] as const).map((r) => (
            <button key={r} className={runtime === r ? "is-on" : ""} onClick={() => setRuntime(r)}>{r}</button>
          ))}
          <button className={leave ? "is-on" : ""} onClick={() => setLeave((v) => !v)}>leave when done</button>
        </div>
      )}
    </>
  );
}

const style = document.createElement("style");
style.textContent = `
.film-stage-frame { position: fixed; inset: 0; }
.film-stage-note { color: #eef3f8; font: 14px var(--mono); text-align: center; margin-top: 40vh; }
.film-stage-bar { position: fixed; top: 10px; right: 12px; z-index: 50; display: flex; gap: 4px; opacity: .35; transition: opacity .2s; }
.film-stage-bar:hover { opacity: 1; }
.film-stage-bar button { font: 11px var(--mono); color: #eef3f8; background: #222; border: 1px solid #444; padding: 3px 8px; cursor: pointer; }
.film-stage-bar button.is-on { border-color: #5ee6c2; color: #5ee6c2; }`;
document.head.appendChild(style);

ReactDOM.createRoot(document.getElementById("root")!).render(<Stage />);
