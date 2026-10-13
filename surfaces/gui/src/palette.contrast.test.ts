import { describe, expect, it } from "vitest";
import { readStyles } from "./testing/source";
import { THEMES } from "./theme";

// The palette's contrast, checked against the stylesheet rather than a copy of it.
//
// `--faint` has now been set too dim twice: once as #4c5a51 (2.74:1) in the phosphor
// palette, and once as #626A71 (3.33:1) when this palette replaced it. Both times the
// mistake was checking the value against `--paper` alone. `paper` is the DARKEST
// surface, so it flatters a dim grey; the binding surface is the lightest one the text
// ever sits on, which is `--solid` — the user bubble.
//
// So this asserts every dim foreground against every surface it can land on, and reads
// the hexes out of styles.css so it fails when the stylesheet changes, not when someone
// remembers to update a fixture.
//
// Every theme in THEMES is checked, by the header of its block: Carbon and Porcelain are
// the two bases, the rest are `[data-palette]` blocks. A theme whose block is missing fails
// here rather than silently painting the base it falls back to.

const CSS = readStyles();

/** Pull a token's value out of a `:root`-ish block, by block header. */
function tokens(blockHeader: string): Record<string, string> {
  const start = CSS.indexOf(blockHeader);
  if (start === -1) throw new Error(`no block ${blockHeader} in styles.css`);
  const body = CSS.slice(start, CSS.indexOf("\n}", start));
  const out: Record<string, string> = {};
  for (const m of body.matchAll(/--([a-z-]+):\s*(#[0-9a-fA-F]{6})\b/g)) out[m[1]] = m[2];
  return out;
}

function luminance(hex: string): number {
  const n = parseInt(hex.slice(1), 16);
  const ch = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * ch[0] + 0.7152 * ch[1] + 0.0722 * ch[2];
}

function contrast(fg: string, bg: string): number {
  const [a, b] = [luminance(fg), luminance(bg)];
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

const AA_TEXT = 4.5;
const AA_NON_TEXT = 3.0;

const header = (id: string) =>
  id === "carbon"
    ? 'html[data-theme="dark"], .theme-preview[data-palette="carbon"] {'
    : id === "porcelain"
      ? ':root, .theme-preview[data-palette="porcelain"] {'
      : `:is(html, .theme-preview)[data-palette="${id}"] {`;

describe.each(THEMES.map((t) => [t.id, header(t.id)]))("%s palette", (_name, header) => {
  const t = tokens(header);
  // Every surface a foreground can sit on — the nav (paper), the chat (canvas), a card
  // (panel), a hover (solid). `solid` is included precisely because it is the lightest in
  // a dark theme and therefore the one that fails first.
  const surfaces = ["paper", "canvas", "panel", "solid"].filter((s) => t[s]);

  it("every text colour clears AA on every surface it can land on", () => {
    const failures: string[] = [];
    for (const fg of ["ink", "muted", "faint", "accent", "ok", "danger", "warn-ink", "teal-ink"]) {
      if (!t[fg]) continue;
      for (const bg of surfaces) {
        const r = contrast(t[fg], t[bg]);
        if (r < AA_TEXT) failures.push(`--${fg} (${t[fg]}) on --${bg} (${t[bg]}): ${r.toFixed(2)}:1`);
      }
    }
    expect(failures, `below ${AA_TEXT}:1:\n  ${failures.join("\n  ")}`).toEqual([]);
  });

  it("text on the accent fill is readable, so the primary button is too", () => {
    expect(contrast(t["on-accent"], t.accent)).toBeGreaterThanOrEqual(AA_TEXT);
  });

  // The accent is the only thing here held to the non-text threshold, and it is
  // held to it because it is load-bearing: it fills the primary button, marks the
  // running process, and is the focus ring. It has to be visible as a *shape*.
  //
  // Dividers deliberately are NOT checked. WCAG's non-text contrast rule covers
  // boundaries that carry meaning — a focus indicator, the only edge telling you
  // where an input is — not decorative separators. `--line` and `--line-strong`
  // sit near 1.3:1 on purpose: hairlines that barely register are the whole point
  // of this design, and forcing them to 3:1 would produce the heavy boxed-in look
  // it exists to avoid. An earlier version of this test asserted it and was wrong.
  it("the accent reads as a shape, not just as text", () => {
    for (const bg of surfaces) {
      expect(contrast(t.accent, t[bg]), `--accent on --${bg}`).toBeGreaterThanOrEqual(
        AA_NON_TEXT,
      );
    }
  });

  // The exception the paragraph above names: a field's outline IS the only edge telling
  // you where the input is. It had been --line, 1.16:1 in dark, on a fill identical to the
  // page. Held to 3:1 on every surface a field sits on.
  it("a form field's outline reads as a boundary", () => {
    expect(t["field-line"], "--field-line is not defined").toBeTruthy();
    for (const bg of surfaces) {
      expect(contrast(t["field-line"], t[bg]), `--field-line on --${bg}`).toBeGreaterThanOrEqual(AA_NON_TEXT);
    }
  });
});
