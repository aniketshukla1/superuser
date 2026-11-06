import { MONO, PALETTE, SANS, mark, type Theme } from "./palette";

// "What's under the hood" as a drawing: the app's eight parts, the audit trail under them, and
// everything outside reached through one door, Egress. Each tile names the folder it lives in
// (under src-tauri/src), so the picture can be checked against the code.
const PARTS: { title: string; what: string; path: string; door?: boolean }[] = [
  { title: "Assistants and skills", what: "20 assistants, 42 skills, each one a markdown file", path: "core/personas · core/skills" },
  { title: "Gateway", what: "Your boundaries, checked before every tool call", path: "core/gateway.rs" },
  { title: "Gates", what: "Invented paths, links and numbers sent back", path: "core/gates" },
  { title: "Memory", what: "Markdown on your disk, history in git", path: "core/brain · core/memory" },
  { title: "Connectors", what: "37 apps with real tools, and any MCP server", path: "integrations" },
  { title: "Scheduler", what: "Routines and watches that report back", path: "core/automation" },
  { title: "A browser each", what: "Hands you the wheel at sign-in pages", path: "core/agentbrowser" },
  { title: "Egress", what: "The one way out. Every request is written down", path: "core/egress.rs", door: true },
];

const OUTSIDE: { title: string; note: string; optional: boolean }[] = [
  { title: "Ollama", note: "on this Mac", optional: false },
  { title: "Your apps", note: "and MCP servers", optional: false },
  { title: "Cloud models", note: "optional", optional: true },
  { title: "The web", note: "optional", optional: true },
];

// Fixed geometry, so the lines below meet the boxes: 1184 inside the card, a 1px border and
// 20px padding in the frame, four columns 12px apart.
const COL = (1184 - 2 - 40 - 36) / 4;
const center = (i: number) => 21 + COL / 2 + i * (COL + 12);

export function architecture(theme: Theme) {
  const c = PALETTE[theme];
  const tile = (p: (typeof PARTS)[number]) => `<div style="box-sizing:border-box;min-height:118px;padding:16px 18px 14px;
      border-radius:12px;background:${c.paper};border:1px solid ${p.door ? c.accent : c.line};display:flex;flex-direction:column;gap:7px">
    <div style="font:600 15.5px/1.2 ${SANS};letter-spacing:-0.01em;color:${p.door ? c.accentInk : c.ink}">${p.title}</div>
    <div style="font:400 13px/1.45 ${SANS};color:${c.muted};flex:1">${p.what}</div>
    <div style="font:400 11px/1 ${MONO};color:${c.faint}">${p.path}</div>
  </div>`;
  const box = (o: (typeof OUTSIDE)[number]) => `<div style="box-sizing:border-box;height:64px;border-radius:12px;
      border:1px ${o.optional ? "dashed" : "solid"} ${theme === "dark" ? "#3a4047" : "#b4bcc2"};display:flex;flex-direction:column;
      align-items:center;justify-content:center;gap:5px;background:${o.optional ? "transparent" : c.panel}">
    <div style="font:600 14.5px/1 ${SANS};color:${c.ink}">${o.title}</div>
    <div style="font:400 11.5px/1 ${MONO};color:${c.faint}">${o.note}</div>
  </div>`;
  const xs = [0, 1, 2, 3].map(center);
  const stroke = theme === "dark" ? "#4a5058" : "#9aa3aa";
  return `<div id="card" style="box-sizing:border-box;width:1280px;padding:40px 48px 44px;background:${c.paper};
      border:1px solid ${c.line};border-radius:16px;font-family:${SANS};color:${c.ink}">
    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:18px">
      <div style="display:flex;align-items:center;gap:16px">${mark(theme, 18)}
        <span style="font:500 15px/1 ${SANS};color:${c.muted}">one desktop app on your Mac</span></div>
      <span style="font:400 12px/1 ${MONO};color:${c.faint}">React interface ⇄ Rust core · 127.0.0.1 · a new token each launch</span>
    </div>
    <div style="box-sizing:border-box;width:1184px;padding:20px;border-radius:16px;background:${c.panel};border:1px solid ${c.line}">
      <div style="display:grid;grid-template-columns:repeat(4,${COL}px);gap:12px">${PARTS.map(tile).join("")}</div>
      <div style="margin-top:12px;padding:12px 16px;border-radius:10px;border:1px solid ${c.line};display:flex;
          justify-content:space-between;font:400 12px/1 ${MONO};color:${c.faint}">
        <span><span style="color:${c.accent};letter-spacing:.08em">AUDIT TRAIL</span> &nbsp; every decision is written down before the action runs</span>
        <span>core/audit</span>
      </div>
    </div>
    <svg width="1184" height="66" viewBox="0 0 1184 66" style="display:block">
      <path d="M${xs[3]} 0V30M${xs[0]} 30H${xs[3]}${xs.map((x) => `M${x} 30V66`).join("")}" fill="none" stroke="${stroke}" stroke-width="1.5"/>
      <circle cx="${xs[3]}" cy="30" r="3.5" fill="${c.accent}"/>
      <text x="${(xs[0] + xs[3]) / 2}" y="22" text-anchor="middle" style="font:400 11.5px ${MONO};fill:${c.faint}">every request out goes through Egress, and is written down</text>
    </svg>
    <div style="box-sizing:border-box;width:1184px;padding:0 21px;display:grid;grid-template-columns:repeat(4,${COL}px);gap:12px">
      ${OUTSIDE.map(box).join("")}
    </div>
  </div>`;
}
