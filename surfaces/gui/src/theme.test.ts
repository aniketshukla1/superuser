import { readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { AUTO_THEME, DEFAULT_THEME, THEMES, getThemePref, resolveTheme, setThemePref, themeMode } from "./theme";
import { readStyles } from "./testing/source";

// The theme list is written three times — THEMES (the picker and the runtime), the palette
// blocks in the stylesheet, and the pre-paint script in index.html, which runs before any
// module can load. A theme in one and not another paints the wrong colours for the first
// frame, or forever. These keep the three in step.

const CSS = readStyles();
const HTML = readFileSync(join(__dirname, "..", "index.html"), "utf8");

const ids = (mode: "dark" | "light") => THEMES.filter((t) => t.mode === mode).map((t) => t.id);
const listInHtml = (name: string) => {
  const m = HTML.match(new RegExp(`var ${name} = \\[([^\\]]*)\\]`));
  return m ? [...m[1].matchAll(/"(\w+)"/g)].map((x) => x[1]) : null;
};

describe("the theme list", () => {
  it("has every theme in the stylesheet, as a base or a palette block", () => {
    const blocks = new Set([...CSS.matchAll(/data-palette="(\w+)"\]\s*\{/g)].map((m) => m[1]));
    expect([...blocks].sort()).toEqual(THEMES.map((t) => t.id).sort());
  });

  it("matches the pre-paint script in index.html, mode for mode", () => {
    expect(listInHtml("dark")).toEqual(ids("dark"));
    expect(listInHtml("light")).toEqual(ids("light"));
  });

  it("resolves Auto to a base of each mode", () => {
    expect(themeMode(AUTO_THEME.dark)).toBe("dark");
    expect(themeMode(AUTO_THEME.light)).toBe("light");
    expect(HTML).toContain(`? "${AUTO_THEME.dark}" : "${AUTO_THEME.light}"`);
  });
});

describe("the stored preference", () => {
  afterEach(() => localStorage.clear());

  // "dark" and "light" are what Settings stored before there were themes. Mapping them
  // keeps an update from changing anybody's choice.
  it("reads the old Dark / Light values as Carbon / Porcelain", () => {
    localStorage.setItem("superuser.theme", "dark");
    expect(getThemePref()).toBe("carbon");
    localStorage.setItem("superuser.theme", "light");
    expect(getThemePref()).toBe("porcelain");
    expect(HTML).toContain('if (t === "dark") t = "carbon";');
    expect(HTML).toContain('if (t === "light") t = "porcelain";');
  });

  // A fresh install, and anything unreadable, is Carbon — in theme.ts and in the first frame.
  it("starts on Carbon, and falls back to it for anything it does not know", () => {
    expect(DEFAULT_THEME).toBe("carbon");
    expect(getThemePref()).toBe("carbon");
    localStorage.setItem("superuser.theme", "solarized");
    expect(getThemePref()).toBe("carbon");
    expect(HTML).toContain('if (dark.indexOf(t) < 0 && light.indexOf(t) < 0) t = "carbon";');
  });

  it("sets the mode and the palette together, and keeps Auto as a choice", () => {
    setThemePref("moss");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.dataset.palette).toBe("moss");
    setThemePref("sand");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.dataset.palette).toBe("sand");
    setThemePref("auto");
    expect(localStorage.getItem("superuser.theme")).toBe("auto");
    expect(getThemePref()).toBe("auto");
    expect(resolveTheme("auto")).toBe(document.documentElement.dataset.palette);
  });
});
