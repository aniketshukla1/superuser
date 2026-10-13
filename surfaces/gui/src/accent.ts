// The accent and the finish — the two things a person tunes on top of a theme.
//
// Accent: the colour of what you press, links, the open tab and the brand mark. "theme" keeps
// each theme's own (ice white on Carbon, the ink on the light ones); anything else is applied over
// every theme as inline custom properties on <html>, after being FITTED to the theme on screen:
// moved toward white on a dark theme, toward black on a light one, until it clears 4.5:1 on all
// four surfaces. So any colour a person picks stays readable, and text on it (near-black on a
// dark theme, white on a light one) clears 4.5:1 by the same arithmetic.
//
// Finish: "flat" paints the accent as a plain fill and titles in the ink; "metal" keeps the
// brushed gradients (styles/accent.css). Both live in localStorage, per device, like the theme.

export const ACCENTS = [
  { id: "icewhite", name: "Ice white", hex: "#DDEAF7" },
  { id: "ice", name: "Ice blue", hex: "#7CC4FF" },
  { id: "bone", name: "Bone", hex: "#EDE4D3" },
  { id: "lime", name: "Electric lime", hex: "#C6F135" },
  { id: "orange", name: "Signal orange", hex: "#FF6B1A" },
  { id: "rose", name: "Rose", hex: "#FF7AA8" },
  { id: "violet", name: "Violet", hex: "#B39DFF" },
  { id: "mint", name: "Mint", hex: "#5EE6C2" },
  { id: "gold", name: "Gold", hex: "#F2C14E" },
] as const;

export type AccentPref = "theme" | (typeof ACCENTS)[number]["id"] | `#${string}`;
export type Finish = "flat" | "metal";

// The default is each theme's own accent — ice white on Carbon, the default theme (owner's
// call, 2026-10-13). "Ice white" is also a preset, to carry it onto any other theme.
export const DEFAULT_ACCENT: AccentPref = "theme";
export const DEFAULT_FINISH: Finish = "flat";
const ACCENT_KEY = "superuser.accent";
const FINISH_KEY = "superuser.finish";
export const ACCENT_EVENT = "superuser:accent-pref";

const isHex = (v: string): v is `#${string}` => /^#[0-9a-fA-F]{6}$/.test(v);
const read = (key: string) => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};
const write = (key: string, v: string) => {
  try {
    localStorage.setItem(key, v);
  } catch {
    /* private mode etc. — still applies for this session */
  }
};

export function getAccentPref(): AccentPref {
  const v = read(ACCENT_KEY);
  if (v === "theme" || ACCENTS.some((a) => a.id === v)) return v as AccentPref;
  return v && isHex(v) ? (v.toUpperCase() as AccentPref) : DEFAULT_ACCENT;
}
export const getFinish = (): Finish => (read(FINISH_KEY) === "metal" ? "metal" : DEFAULT_FINISH);

/** The colour a preference names, before fitting; null for "theme". */
export const accentHex = (pref: AccentPref): string | null =>
  pref === "theme" ? null : isHex(pref) ? pref : ACCENTS.find((a) => a.id === pref)!.hex;

// ── contrast ────────────────────────────────────────────────────────────────
const rgb = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const hex = (c: number[]) => "#" + c.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("").toUpperCase();
const lum = (h: string) =>
  rgb(h)
    .map((v) => (v / 255 <= 0.03928 ? v / 255 / 12.92 : ((v / 255 + 0.055) / 1.055) ** 2.4))
    .reduce((s, c, i) => s + c * [0.2126, 0.7152, 0.0722][i], 0);
export const contrast = (a: string, b: string) => {
  const [x, y] = [lum(a), lum(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
};

/** `share` of `color` over `base`, in sRGB — what CSS `color-mix(in srgb, …)` paints. */
const mixOver = (color: string, base: string, share: number) =>
  hex(rgb(base).map((v, i) => v + (rgb(color)[i] - v) * share));

/** Move `color` toward white (dark theme) or black (light theme) until it clears 4.5:1 on every
 * surface ([paper, canvas, panel, solid]) AND on its own soft tint — a count or a selected chip
 * sets the accent on that tint, and it was the one background the first fit forgot. Then the
 * text that sits on it. */
export function fitAccent(color: string, surfaces: string[], mode: "dark" | "light") {
  const to = mode === "dark" ? [255, 255, 255] : [0, 0, 0];
  let fitted = color.toUpperCase();
  let soft = mixOver(fitted, surfaces[2], 0.14);
  for (let t = 0; t <= 1.0001; t += 0.02) {
    fitted = hex(rgb(color).map((v, i) => v + (to[i] - v) * t));
    soft = mixOver(fitted, surfaces[2], 0.14);
    if ([...surfaces, soft].every((s) => contrast(fitted, s) >= 4.5)) break;
  }
  return { accent: fitted, accentSoft: soft, onAccent: mode === "dark" ? "#0B0B0B" : "#FFFFFF" };
}

// ── applying ────────────────────────────────────────────────────────────────
const PROPS = ["--accent", "--on-accent", "--accent-soft"];

/** Paint the stored accent and finish over the theme on screen. Called by theme.ts whenever
 * the theme changes, since the fit depends on the theme's surfaces. */
export function applyAccent() {
  const root = document.documentElement;
  root.dataset.finish = getFinish();
  const color = accentHex(getAccentPref());
  root.dataset.accent = color ? "set" : "theme";
  if (!color) {
    PROPS.forEach((p) => root.style.removeProperty(p));
    return;
  }
  const css = getComputedStyle(root);
  const surfaces = ["--paper", "--canvas", "--panel", "--solid"].map((p) => css.getPropertyValue(p).trim());
  // Before the stylesheet has loaded there is nothing to fit against; try again once it has.
  if (surfaces.some((s) => !isHex(s))) {
    window.addEventListener("load", applyAccent, { once: true });
    return;
  }
  const { accent, accentSoft, onAccent } = fitAccent(color, surfaces, root.dataset.theme === "dark" ? "dark" : "light");
  root.style.setProperty("--accent", accent);
  root.style.setProperty("--on-accent", onAccent);
  root.style.setProperty("--accent-soft", accentSoft);
}

export function setAccentPref(pref: AccentPref) {
  write(ACCENT_KEY, pref);
  applyAccent();
  window.dispatchEvent(new CustomEvent(ACCENT_EVENT));
}
export function setFinish(f: Finish) {
  write(FINISH_KEY, f);
  applyAccent();
  window.dispatchEvent(new CustomEvent(ACCENT_EVENT));
}
