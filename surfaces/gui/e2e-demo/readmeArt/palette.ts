// The README art's two palettes: the app's own dark and light surfaces with the website's
// periwinkle accent. GitHub shows one per reader, through <picture> in the README.
export type Theme = "dark" | "light";

export const PALETTE = {
  dark: { paper: "#0b0d0f", panel: "#121517", ink: "#e8eaed", muted: "#8b9299", faint: "#788188", line: "#22262a", accent: "#8a8cf0", accentInk: "#b9bbff" },
  light: { paper: "#f4f6f7", panel: "#ffffff", ink: "#14181b", muted: "#545e66", faint: "#656e75", line: "#c9d0d5", accent: "#5457c8", accentInk: "#3f42b0" },
} as const;

// The app bundles both faces (src/main.tsx), so the page the spec draws on already has them.
export const SANS = "Inter, system-ui, sans-serif";
export const MONO = "'JetBrains Mono', monospace";

/** The wordmark: the accent `#`, a block cursor, the name. */
export function mark(theme: Theme, size: number) {
  const c = PALETTE[theme];
  return `<div style="display:flex;align-items:center;gap:${size * 0.55}px;font:500 ${size}px/1 ${MONO};letter-spacing:-0.01em;color:${c.ink}">
    <span style="display:inline-flex;align-items:center;gap:${size * 0.18}px"><span style="color:${c.accent}">#</span><span style="display:inline-block;width:0.42em;height:0.92em;background:${c.ink};border-radius:1px"></span></span>
    <span>superuser</span>
  </div>`;
}
