import { MONO, PALETTE, SANS, type Theme } from "./palette";

// The README's buttons and its numbers strip, drawn in the app's own type rather than taken
// from a badge service. One button image serves both of GitHub's themes: GitHub moves an image
// out of its link when a <picture> sits inside an <a>, so a linked button can't switch by theme.

const ICON = {
  play: `<svg width="13" height="14" viewBox="0 0 13 14"><path d="M1.5 1.6v10.8c0 .8.9 1.3 1.6.9l8.6-5.4c.6-.4.6-1.3 0-1.7L3.1.7c-.7-.4-1.6.1-1.6.9z" fill="currentColor"/></svg>`,
  arrow: `<svg width="15" height="14" viewBox="0 0 15 14"><path d="M1 7h12M8.5 2.5 13 7l-4.5 4.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
  out: `<svg width="13" height="13" viewBox="0 0 13 13"><path d="M4 1.5h7.5V9M11.2 1.8 1.5 11.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
};

export type Button = { file: string; label: string; icon: keyof typeof ICON; primary: boolean; lead?: boolean };

export const BUTTONS: Button[] = [
  { file: "btn-film", label: "Watch the film", icon: "play", primary: true, lead: true },
  { file: "btn-start", label: "Get started", icon: "arrow", primary: false },
  { file: "btn-site", label: "Website", icon: "out", primary: false },
];

export function button(b: Button) {
  const c = PALETTE.dark;
  const look = b.primary
    ? "background:linear-gradient(180deg,#7679ef,#4f52c9);color:#fff;border:1px solid rgba(255,255,255,.14);" +
      "box-shadow:inset 0 1px 0 rgba(255,255,255,.28)"
    : `background:#16191c;color:${c.ink};border:1px solid #30363c`;
  const icon = `<span style="display:inline-flex;opacity:${b.primary ? 1 : 0.7}">${ICON[b.icon]}</span>`;
  return `<div id="card" style="display:inline-flex;align-items:center;gap:10px;box-sizing:border-box;height:46px;
      padding:0 ${b.lead ? 20 : 22}px 0 ${b.lead ? 18 : 22}px;border-radius:999px;${look};
      font:600 15px/1 ${SANS};letter-spacing:-0.005em;white-space:nowrap">
    ${b.lead ? icon + b.label : b.label + icon}
  </div>`;
}

/** Counted from the source on 2026-11-07; README.md says where each one comes from. */
export const NUMBERS: [string, string][] = [
  ["20", "built-in assistants"],
  ["42", "built-in skills"],
  ["37", "connectors + any MCP"],
  ["14", "model providers"],
  ["1,600+", "Rust tests"],
  ["0", "analytics calls"],
];

export function stats(theme: Theme) {
  const c = PALETTE[theme];
  const cells = NUMBERS.map(
    ([n, label], i) => `<div style="padding:4px 22px;${i ? `border-left:1px solid ${c.line};` : ""}">
      <div style="font:600 46px/1 ${SANS};letter-spacing:-0.035em;color:${n === "0" ? c.accent : c.ink}">${n}</div>
      <div style="margin-top:14px;font:500 11.5px/1.35 ${MONO};letter-spacing:0.09em;text-transform:uppercase;color:${c.faint}">${label}</div>
    </div>`,
  ).join("");
  return `<div id="card" style="box-sizing:border-box;width:1280px;padding:34px 26px;display:grid;
      grid-template-columns:repeat(6,minmax(0,1fr));background:${c.panel};
      border:1px solid ${c.line};border-radius:16px;font-family:${SANS}">${cells}</div>`;
}
