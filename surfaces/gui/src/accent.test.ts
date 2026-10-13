import { afterEach, describe, expect, it } from "vitest";
import { ACCENTS, DEFAULT_ACCENT, accentHex, contrast, fitAccent, getAccentPref, getFinish } from "./accent";
import { THEMES } from "./theme";
import { readStyles } from "./testing/source";

// Settings promises that ANY accent works. This holds it to that: every preset and a spread of
// awkward custom colours, fitted to every theme's surfaces, must read as text on all four and on
// its own soft tint (where counts and selected chips sit), and the text that sits on the accent
// must read on it — flat, and at the extreme stop of the metal.

const CSS = readStyles();
const header = (id: string) =>
  id === "carbon"
    ? 'html[data-theme="dark"], .theme-preview[data-palette="carbon"] {'
    : id === "porcelain"
      ? ':root, .theme-preview[data-palette="porcelain"] {'
      : `:is(html, .theme-preview)[data-palette="${id}"] {`;
function surfaces(id: string): string[] {
  const start = CSS.indexOf(header(id));
  const body = CSS.slice(start, CSS.indexOf("\n}", start));
  return ["paper", "canvas", "panel", "solid"].map((k) => body.match(new RegExp(`--${k}:\\s*(#[0-9A-Fa-f]{6})`))![1]);
}
/** sRGB `color-mix(in srgb, c, black p)` — what the metal finish paints. */
const shade = (c: string, p: number) =>
  "#" + [1, 3, 5].map((i) => Math.round(parseInt(c.slice(i, i + 2), 16) * (1 - p)).toString(16).padStart(2, "0")).join("");

const CUSTOM = ["#000000", "#FFFFFF", "#808080", "#1A237E", "#FFFF00", "#00FF00", "#FF0000", "#3D2B1F"];
const COLOURS = [...ACCENTS.map((a) => a.hex), ...CUSTOM];

describe.each(THEMES.map((t) => [t.id, t.mode] as const))("an accent fitted to %s", (id, mode) => {
  const s = surfaces(id);
  it.each(COLOURS)("%s reads on every surface, and text reads on it", (c) => {
    const { accent, accentSoft, onAccent } = fitAccent(c, s, mode);
    for (const bg of [...s, accentSoft]) expect(contrast(accent, bg), `${accent} on ${bg}`).toBeGreaterThanOrEqual(4.5);
    expect(contrast(onAccent, accent)).toBeGreaterThanOrEqual(4.5);
    // Metal: the darkest band on a dark theme (8% black), the lightest stop on a light one.
    const extreme = mode === "dark" ? shade(accent, 0.08) : accent;
    expect(contrast(onAccent, extreme), `text on the metal's ${extreme}`).toBeGreaterThanOrEqual(4.5);
  });
});

describe("the stored accent", () => {
  afterEach(() => localStorage.clear());

  // The default is the theme's own accent — ice white on Carbon, the default theme.
  it("defaults to the theme's own accent and a flat finish", () => {
    expect(getAccentPref()).toBe(DEFAULT_ACCENT);
    expect(DEFAULT_ACCENT).toBe("theme");
    expect(accentHex(getAccentPref())).toBeNull();
    expect(getFinish()).toBe("flat");
  });

  it("has Ice white first, the same colour as Carbon's own", () => {
    expect(ACCENTS[0]).toMatchObject({ id: "icewhite", hex: "#DDEAF7" });
    expect(CSS).toMatch(/--accent: #DDEAF7;/);
  });

  it("keeps a preset, a custom colour, or the theme's own", () => {
    localStorage.setItem("superuser.accent", "lime");
    expect(getAccentPref()).toBe("lime");
    localStorage.setItem("superuser.accent", "#12ab9c");
    expect(getAccentPref()).toBe("#12AB9C");
    localStorage.setItem("superuser.accent", "theme");
    expect(accentHex(getAccentPref())).toBeNull();
  });

  it("falls back to the default for anything it does not know", () => {
    localStorage.setItem("superuser.accent", "chartreuse");
    expect(getAccentPref()).toBe(DEFAULT_ACCENT);
  });
});
