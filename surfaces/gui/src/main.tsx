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

import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import Pet from "./pet/Pet";
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
// Platform hook for CSS (html[data-platform="windows"] scrollbar styling etc.).
document.documentElement.dataset.platform = platformOS();

// A file dropped OUTSIDE a drop target (the composer) must never navigate the webview to the
// file itself — the browser/WKWebView default. Drop targets stopPropagation-free preventDefault
// in their own handlers; these guards only catch the misses. (The desktop shell disables Tauri's
// native drag-drop interception so HTML5 drag events reach the DOM at all — see lib.rs.)
window.addEventListener("dragover", (e) => e.preventDefault());
window.addEventListener("drop", (e) => e.preventDefault());

// Cards catch a soft light under the pointer (the dark theme's spotlight in styles.css).
// One listener for the whole app; it only moves two custom properties on the card.
document.addEventListener("pointermove", (e) => {
  const card = (e.target as Element | null)?.closest?.<HTMLElement>(".chart-block, .rounded-xl2");
  if (!card) return;
  const r = card.getBoundingClientRect();
  card.style.setProperty("--mx", `${e.clientX - r.left}px`);
  card.style.setProperty("--my", `${e.clientY - r.top}px`);
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isPet ? <Pet /> : <App />}</React.StrictMode>,
);
