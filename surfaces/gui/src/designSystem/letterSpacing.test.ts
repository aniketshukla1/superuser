import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { describe, expect, it } from "vitest";
import { readModule } from "../testing/source";
import { CONFIG, CSS, SRC, px, tsFiles, tsxFiles } from "./designSystem.helpers";


describe("letter-spacing", () => {
  // One tightening value. The stylesheet used -0.01em in three places while
  // thirteen components used Tailwind's `tracking-tight` at -0.025em — so a
  // markdown h1 and a page h1, both --t-xl, were tracked 2.5x differently. Nobody
  // chose that; -0.025em is a framework default that nothing here ever picked.
  it("has one token and points Tailwind at it", () => {
    expect(() => px("--ls-tight")).toThrow(); // it is an em value, not px
    expect(CSS, "--ls-tight is not defined").toMatch(/--ls-tight:\s*-?[\d.]+em/);
    expect(CONFIG, "tracking-tight does not point at the token").toContain("var(--ls-tight)");
  });

  it("is the only source of letter-spacing on headings", () => {
    // Positive tracking on small caps labels is a different thing and stays.
    const negatives = [...CSS.matchAll(/letter-spacing:\s*(-[\d.]+em)/g)]
      .map((m) => m[1])
      // The "how it works" illustration draws a miniature of the app: a 96px rail
      // with a 9px wordmark in it. That is a picture of an interface, not the
      // interface — its type is off the scale by design, the way the boot caret's
      // 1px radius is. Same exemption, same reason.
      .filter((v) => v !== "-.015em");
    expect(negatives, `hand-written tightening in styles.css: ${negatives.join(", ")}`).toEqual([]);
  });
});

describe("colour", () => {
  // The palette is three deliberate foreground steps — --ink, --muted (6.1:1 on
  // paper), --faint — and palette.contrast.test.ts checks each against every
  // surface it can land on, reading the hexes out of styles.css.
  //
  // `text-ink/90` invents a fourth step that is computed at render time and never
  // appears in the stylesheet, so it is outside that check entirely. --faint has
  // been set too dim twice already; a shade nobody measured is how it happens a
  // third time.
  // Six rules painted failures in the accent — a failed run, an MCP error, a refused tool —
  // and on this palette the accent means "the thing to press". A failure that looks like a
  // call to action is the one colour mistake a person acts on.
  it("paints failures in --danger, never in the accent", () => {
    const offenders = [...CSS.matchAll(/(^|\n)([^\n{}]*(?:error|danger|denied|fail)[^\n{}]*)\{([^}]*)\}/g)]
      .filter((m) => /(?<![-\w])color:\s*var\(--accent\)/.test(m[3]))
      .map((m) => m[2].trim());
    expect(offenders, `failure states in the accent colour:\n${offenders.join("\n")}`).toEqual([]);
  });

  it("has no ad-hoc opacity shades of a foreground token", () => {
    const offenders: string[] = [];
    for (const f of [...tsxFiles(), ...tsFiles()]) {
      for (const m of readFileSync(f, "utf8").matchAll(/text-(ink|muted|faint)\/\d+/g)) {
        offenders.push(`${basename(f)}: ${m[0]}`);
      }
    }
    expect(
      offenders,
      `Use a palette step (text-ink / text-muted / text-faint). An opacity shade is a ` +
        `colour nobody contrast-checked, because the contrast test reads the stylesheet.\n` +
        offenders.join("\n"),
    ).toEqual([]);
  });
});

describe("variables", () => {
  // A var() that names nothing does not error. It computes to the property's initial value,
  // so the element renders — just wrong. The feedback panel read `--raised` and `--r-card`
  // for its fill and corners, neither of which ever existed, and shipped transparent and
  // square. The assistant panel read `--fg` and `--bg` from a palette two redesigns ago.
  // Everything rendered; nothing was right.
  //
  // Defined means: declared in the stylesheet, or set at runtime from a component — a
  // custom property passed as a string key (`{ "--brand": colour }`), which is how the
  // connector card and the nav width reach the stylesheet.
  it("reads no custom property that nothing defines", () => {
    const defined = new Set([...CSS.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]));
    for (const f of [...tsxFiles(), ...tsFiles()]) {
      for (const m of readFileSync(f, "utf8").matchAll(/["'`](--[\w-]+)["'`]/g)) defined.add(m[1]);
    }
    const missing = [...new Set([...CSS.matchAll(/var\((--[\w-]+)/g)].map((m) => m[1]))].filter((n) => !defined.has(n));
    expect(missing, `var() with no definition: ${missing.join(", ")}`).toEqual([]);
  });

  it("gives palette tokens no fallback colour of their own", () => {
    // `var(--danger, #e5484d)` is a second danger red that only appears when the first is
    // missing — which is exactly when nobody is looking. A token that exists needs no
    // fallback; one that does not should fail the test above, not paint a guess.
    const fallbacks = [...CSS.matchAll(/var\((--(?:paper|panel|ink|muted|faint|line|line-strong|accent|on-accent|ok|danger|warn-ink|solid)),\s*[^)]+\)/g)].map(
      (m) => m[0],
    );
    expect(fallbacks, fallbacks.join(", ")).toEqual([]);
  });
});

describe("the sidebar", () => {
  // The app's densest settled surface, and the one the rest of the system quotes:
  // 18px line box with 4px either side is a 26px row, and that rhythm is where the
  // control height came from. Being the reference is exactly why it should not be
  // the one place nobody checks — it had five 12.5px labels and two below 10px.
  const SIDEBAR = readModule(join(SRC, "components", "Sidebar.tsx"));

  it("uses only sizes that are on the type scale", () => {
    const steps = ["xs", "sm", "md", "lg", "xl"].map((s) => px(`--t-${s}`));
    const used = [...SIDEBAR.matchAll(/text-\[([0-9.]+)px\]/g)].map((m) => Number(m[1]));
    const off = [...new Set(used.filter((n) => !steps.includes(n)))].sort((a, b) => a - b);
    expect(off, `sizes off the scale in the sidebar: ${off.join(", ")}px`).toEqual([]);
  });

  it("keeps its rows on one rhythm", () => {
    // The 26px row is stated once, as a line box plus padding, and the comment that
    // records the arithmetic has been load-bearing twice. The padding is --s-2 since the
    // spacing sweep, so this asserts the token AND the number behind it — pinning only
    // the token would let the row change height when the scale moved.
    expect(CSS).toMatch(
      /\.sidebar \.list-row \{ line-height: 18px; padding-top: var\(--s-2\); padding-bottom: var\(--s-2\); \}/,
    );
    expect(px("--t-md-lh")).toBe(18);
    expect(px("--s-2")).toBe(4);
  });
});
