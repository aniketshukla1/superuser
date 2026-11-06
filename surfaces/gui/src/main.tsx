// Bundled, not fetched. `--sans` and `--mono` name Inter and JetBrains
// Mono; loading either from a font CDN would put a network request on every
// launch of an app whose whole claim is that it runs on your machine. These
// resolve from node_modules at build time, so the app still starts offline.
// Only the weights the type scale uses — 400 body, 500 labels, 600 titles — plus one.
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
// 800 for one thing: the launch film's name (components/LaunchFilm), shown once per launch.
import "@fontsource/inter/800.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import Pet from "./pet/Pet";
import { App } from "./App";
import { initTheme } from "./theme";
import { platformOS } from "./tauri";
import { startTestBridge } from "./testbridge";
import { initTooltips } from "./tooltip";
import "./tailwind.css";
import "./styles.css";

// Every import above has loaded and run. index.html watches for this flag: until it is set,
// the pre-boot screen stays up and reports the first error it saw instead of a blank window.
(window as unknown as { __superuserBooted?: boolean }).__superuserBooted = true;

// The Rust shell opens a second, transparent window with ?mascot — render just the
// floating pet there (no app chrome, no HTTP bridge) instead of the full App.
const isPet = new URLSearchParams(window.location.search).has("mascot");
if (isPet) document.documentElement.classList.add("mascot-window");

// Dev builds only, and outbound-only: dials the test runner if one is listening on
// loopback, otherwise does nothing. Stripped from production bundles. See testbridge.ts.
startTestBridge();

initTheme();
initTooltips();
// Lets CSS tell platforms apart (html[data-platform="windows"] gets its own scrollbars, for one).
document.documentElement.dataset.platform = platformOS();

// A file dropped anywhere but a drop target (the composer) would otherwise send the webview to
// the file itself, which is what browsers and WKWebView do by default. Drop targets call
// preventDefault in their own handlers; these two catch the rest. (The desktop shell turns off
// Tauri's own drag-and-drop handling so HTML5 drag events reach the page at all; see lib.rs.)
for (const type of ["dragover", "drop"] as const) window.addEventListener(type, (e) => e.preventDefault());

// Cards catch a soft light under the pointer (the dark theme's spotlight in styles.css).
// One listener for the whole app; it only moves two custom properties on the card.
document.addEventListener("pointermove", (e) => {
  const card = (e.target as Element | null)?.closest?.<HTMLElement>(".chart-block, .rounded-xl2");
  if (!card) return;
  const r = card.getBoundingClientRect();
  card.style.setProperty("--mx", `${e.clientX - r.left}px`);
  card.style.setProperty("--my", `${e.clientY - r.top}px`);
});

createRoot(document.getElementById("root")!).render(<StrictMode>{isPet ? <Pet /> : <App />}</StrictMode>);
