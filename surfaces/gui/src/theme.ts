// Appearance: a colour theme, or Auto (follow macOS); Carbon until someone picks. The
// preference lives in localStorage only — it's per-device (like macOS appearance itself) and must apply before the sidecar
// is even reachable. index.html applies it inline pre-paint with the same key and the same
// lists, so the first frame is already the right colour; this module keeps it current.
//
// Two attributes on <html>: `data-theme` is the MODE ("dark" | "light") that every
// mode-specific rule in styles.css keys on, and `data-palette` names the theme, whose
// colours are a `[data-palette]` block (Carbon and Porcelain are the two bases).
import { useEffect, useState } from "react";
import { applyAccent } from "./accent";

export type ThemeMode = "dark" | "light";

/** The family of themes, in the order the picker shows them. */
export const THEMES = [
  { id: "carbon", name: "Carbon", mode: "dark", note: "matte black and ice white" },
  { id: "graphite", name: "Graphite", mode: "dark", note: "soft dark" },
  { id: "slate", name: "Slate", mode: "dark", note: "cool blue-grey" },
  { id: "fjord", name: "Fjord", mode: "dark", note: "arctic, softer dark" },
  { id: "abyss", name: "Abyss", mode: "dark", note: "deep navy" },
  { id: "moss", name: "Moss", mode: "dark", note: "green-grey" },
  { id: "umber", name: "Umber", mode: "dark", note: "warm brown-black" },
  { id: "dusk", name: "Dusk", mode: "dark", note: "violet-grey" },
  { id: "porcelain", name: "Porcelain", mode: "light", note: "white and graphite" },
  { id: "paper", name: "Paper", mode: "light", note: "warm off-white" },
  { id: "mist", name: "Mist", mode: "light", note: "cool grey" },
  { id: "sand", name: "Sand", mode: "light", note: "sepia, low glare" },
  { id: "sage", name: "Sage", mode: "light", note: "green-tinted" },
  { id: "frost", name: "Frost", mode: "light", note: "pale blue" },
  { id: "dawn", name: "Dawn", mode: "light", note: "rose-tinted" },
  { id: "chalk", name: "Chalk", mode: "light", note: "highest contrast" },
] as const satisfies readonly { id: string; name: string; mode: ThemeMode; note: string }[];

export type ThemeId = (typeof THEMES)[number]["id"];
export type ThemePref = ThemeId | "auto";

/** What Auto resolves to, by the Mac's appearance. */
export const AUTO_THEME: Record<ThemeMode, ThemeId> = { dark: "carbon", light: "porcelain" };

/** What a fresh install shows, and what anything unreadable falls back to (owner's call,
 * 2026-10-13: Carbon, not Auto). Auto is a choice in Settings, stored like any other. */
export const DEFAULT_THEME: ThemePref = "carbon";

const KEY = "superuser.theme";
const PREF_EVENT = "superuser:theme-pref";
// The two values stored before there were themes. Mapped, not dropped, so an update never
// changes what somebody chose.
const LEGACY: Record<string, ThemeId> = { dark: "carbon", light: "porcelain" };
const media = window.matchMedia?.("(prefers-color-scheme: dark)");

export const isThemeId = (v: unknown): v is ThemeId => THEMES.some((t) => t.id === v);
export const themeMode = (id: ThemeId): ThemeMode => THEMES.find((t) => t.id === id)!.mode;

export function getThemePref(): ThemePref {
  try {
    const raw = localStorage.getItem(KEY);
    const v = raw && LEGACY[raw] ? LEGACY[raw] : raw;
    return v === "auto" || isThemeId(v) ? v : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

/** The theme a preference shows right now. */
export function resolveTheme(pref: ThemePref): ThemeId {
  return pref === "auto" ? AUTO_THEME[media?.matches ? "dark" : "light"] : pref;
}

function apply(pref: ThemePref) {
  const id = resolveTheme(pref);
  const root = document.documentElement;
  root.dataset.theme = themeMode(id);
  root.dataset.palette = id;
  // The accent is fitted to the theme's surfaces, so it is re-fitted with every theme.
  applyAccent();
}

export function setThemePref(pref: ThemePref) {
  try {
    localStorage.setItem(KEY, pref);
  } catch {
    /* private mode etc. — still applies for this session */
  }
  apply(pref);
  window.dispatchEvent(new CustomEvent(PREF_EVENT));
}

/** Call once at startup: applies the stored pref and follows macOS appearance while in auto. */
export function initTheme() {
  apply(getThemePref());
  media?.addEventListener("change", () => {
    if (getThemePref() === "auto") {
      apply("auto");
      window.dispatchEvent(new CustomEvent(PREF_EVENT));
    }
  });
}

/** The settings control's hook — stays in sync if the pref changes elsewhere. */
export function useThemePref(): [ThemePref, (p: ThemePref) => void] {
  const [pref, setPref] = useState<ThemePref>(getThemePref);
  useEffect(() => {
    const sync = () => setPref(getThemePref());
    window.addEventListener(PREF_EVENT, sync);
    return () => window.removeEventListener(PREF_EVENT, sync);
  }, []);
  return [pref, setThemePref];
}

/** The theme on screen right now, kept current — for code that draws its own colours (the
 * code editor, the terminal) and has to follow the theme rather than the Mac. */
export function useTheme(): ThemeId {
  const read = (): ThemeId => {
    const id = document.documentElement.dataset.palette;
    return isThemeId(id) ? id : resolveTheme(getThemePref());
  };
  const [id, setId] = useState<ThemeId>(read);
  useEffect(() => {
    const sync = () => setId(read());
    window.addEventListener(PREF_EVENT, sync);
    return () => window.removeEventListener(PREF_EVENT, sync);
  }, []);
  return id;
}
