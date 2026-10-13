import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { describe, expect, it } from "vitest";
import { readModule } from "../testing/source";
import { CONFIG, CSS, SRC, px, tsFiles, tsxFiles } from "./designSystem.helpers";


describe("the scales live in one place", () => {
  it("defines every token in the stylesheet", () => {
    for (const n of ["--r-xs", "--r-sm", "--r-md", "--r-lg", "--t-xs", "--t-sm", "--t-md", "--t-lg", "--t-xl", "--ctl-h", "--ctl-pad-y"]) {
      expect(() => px(n), `${n} missing`).not.toThrow();
    }
  });

  it("has Tailwind point at them rather than hold a second copy", () => {
    // The same reasoning that moved fontFamily onto the variables after `--sans`
    // changed one thing and left every `font-sans` utility on the old typeface:
    // a second copy of a value is a second thing to forget.
    const block = (key: string) => CONFIG.slice(CONFIG.indexOf(key), CONFIG.indexOf("}", CONFIG.indexOf(key)));
    expect(block("borderRadius:")).not.toMatch(/\d+px/);
    expect(block("fontSize:")).not.toMatch(/\d+px/);
    expect(CONFIG).toContain("var(--r-lg)");
    expect(CONFIG).toContain("var(--t-md)");
  });

  it("keeps Tailwind's own radius names on the scale too", () => {
    // `rounded-lg` is used 106 times and was Tailwind's default 8px — a number
    // nobody here chose. `rounded-xl` (24 uses) was 12px, off the scale entirely.
    for (const key of ["lg:", "xl:", '"2xl":', "md:", "sm:"]) {
      const at = CONFIG.indexOf(key, CONFIG.indexOf("borderRadius:"));
      expect(CONFIG.slice(at, at + 40), `rounded-${key} is off the scale`).toMatch(/var\(--r-/);
    }
  });
});

describe("radius", () => {
  it("orders smallest chrome to biggest box", () => {
    // Backwards is the specific thing that reads as "off": a key cap rounder than
    // the card it sits in.
    expect(px("--r-xs")).toBeLessThan(px("--r-sm"));
    expect(px("--r-sm")).toBeLessThan(px("--r-md"));
    expect(px("--r-md")).toBeLessThan(px("--r-lg"));
    // Nothing rounder than the largest box: measured against the owner's
    // reference, the artifact's values ran ~38% too round.
    expect(px("--r-lg")).toBeLessThanOrEqual(12);
  });

  it("is the only source of radius in the stylesheet", () => {
    const raw = [...CSS.matchAll(/border-radius:\s*([^;]+);/g)]
      .map((m) => m[1].trim())
      .filter((v) => !v.includes("var(") && v !== "0")
      // The boot caret is a 7px-wide blinking bar; the smallest step would round
      // it into a lozenge. A hairline, not a corner — and documented as such.
      .filter((v) => v !== "1px");
    expect(raw, `hand-written radii in styles.css: ${raw.join(", ")}`).toEqual([]);
  });
});

describe("type", () => {
  it("pairs every size with a line-height", () => {
    // `text-[12.5px]` sets the size and NOT the line-height. That is not a quirk
    // to work around — it is why ~400 call sites had emergent row heights.
    for (const step of ["xs", "sm", "md", "lg", "xl"]) {
      expect(() => px(`--t-${step}`)).not.toThrow();
      expect(() => px(`--t-${step}-lh`), `--t-${step} has no line-height`).not.toThrow();
    }
  });

  it("keeps line-heights on whole pixels", () => {
    // A unitless 1.45 on a 12.5px font is 18.125px, which rounds per-element — so
    // the same "paragraph gap" measures differently in two places. That is what
    // made the cards read loose.
    for (const step of ["xs", "sm", "md", "lg", "xl"]) {
      expect(px(`--t-${step}-lh`) % 1, `--t-${step}-lh is fractional`).toBe(0);
      expect(px(`--t-${step}`) % 1, `--t-${step} is a half-pixel size`).toBe(0);
    }
  });

  it("keeps every step readable and in order", () => {
    const sizes = ["xs", "sm", "md", "lg", "xl"].map((s) => px(`--t-${s}`));
    expect(sizes).toEqual([...sizes].sort((a, b) => a - b));
    expect(Math.min(...sizes), "nothing below 11px — it stops being readable").toBeGreaterThanOrEqual(11);
    // Line-height has to leave room for descenders at every step.
    for (const s of ["xs", "sm", "md", "lg", "xl"]) {
      expect(px(`--t-${s}-lh`) / px(`--t-${s}`)).toBeGreaterThanOrEqual(1.25);
    }
  });
});

describe("type, everywhere", () => {
  // The scale existed and the sidebar was checked against it; nothing else was. So
  // 725 call sites across 50 files set a size with `text-[13px]` and no
  // line-height — the precise failure the `--t-*` pairs were introduced to end,
  // reintroduced one reasonable local decision at a time, in the one place a
  // convention cannot reach and a test can.
  //
  // Checked over every .tsx and .ts, because the drift was not in one folder.
  it("sets sizes through the scale, not as raw pixels", () => {
    const files = [...tsxFiles(), ...tsFiles()];
    const offenders = files
      .map((f) => [f, [...readFileSync(f, "utf8").matchAll(/text-\[[0-9.]+px\]/g)].map((m) => m[0])] as const)
      .filter(([, hits]) => hits.length > 0)
      .map(([f, hits]) => `${basename(f)}: ${[...new Set(hits)].join(", ")}`);

    expect(
      offenders,
      `Use the scale — text-xs/sm/md/lg/xl — not a raw pixel size.\n` +
        `A bracket size sets font-size and NOT line-height, so the row height becomes ` +
        `whatever the browser computes, and two "same" rows measure differently.\n` +
        offenders.join("\n"),
    ).toEqual([]);
  });

  it("keeps prose on the whole-pixel line-height, not Tailwind's unitless one", () => {
    // `leading-relaxed` is a unitless 1.625: 24.375px on a 15px font. A fractional
    // line box rounds per element, so two paragraphs at the "same" size measure
    // differently — the exact failure the --t-* pairs were introduced to end, and
    // the reason --t-prose-lh exists. It had no Tailwind name, so 28 call sites
    // reached for the nearest default instead.
    const files = [...tsxFiles(), ...tsFiles()];
    const offenders = files
      .filter((f) => /leading-relaxed/.test(readFileSync(f, "utf8")))
      .map((f) => basename(f));
    expect(offenders, `use leading-prose (a whole-pixel token) rather than leading-relaxed:\n${offenders.join("\n")}`).toEqual([]);
    expect(CONFIG).toContain("var(--t-prose-lh)");
  });

  it("has a utility for every step of the scale", () => {
    // If a step has no Tailwind name, the only way to reach it is a bracket — and
    // the rule above would be asking people to do something impossible.
    for (const step of ["xs", "sm", "md", "lg", "xl"]) {
      expect(CONFIG, `text-${step} has no mapping`).toMatch(new RegExp(`${step}:\\s*\\["var\\(--t-${step}\\)"`));
    }
  });
});

describe("shared class names", () => {
  // Sixteen declarations across ten files for four ideas: CARD nine times, SEC_H
  // four, FIELD_LABEL twice. The same shape as the type drift, one level up — a
  // reasonable local decision made with nowhere to look the answer up — and it had
  // already produced its first divergence before anyone noticed.
  //
  // The test compares VALUES, not names, so a genuinely different card (the
  // translucent one inside GalleryModal) is still allowed to exist and say why.
  const SHARED = readModule(join(SRC, "ui.ts"));
  const values = [...SHARED.matchAll(/export const (\w+) = ("[^"]+")/g)].map((m) => ({ name: m[1], value: m[2] }));

  it("defines the shared ones in one place", () => {
    expect(values.map((v) => v.name).sort()).toEqual(["CARD", "FIELD_HELP", "FIELD_LABEL", "SEC_H"]);
  });

  it("has no second copy of a shared value", () => {
    const offenders: string[] = [];
    for (const f of [...tsxFiles(), ...tsFiles()]) {
      if (basename(f) === "ui.ts") continue;
      const src = readFileSync(f, "utf8");
      for (const { name, value } of values) {
        if (src.includes(`= ${value}`)) offenders.push(`${basename(f)} redeclares ${name}`);
      }
    }
    expect(offenders, `import these from ../ui rather than copying them:\n${offenders.join("\n")}`).toEqual([]);
  });

  it("keeps section headings off hand-rolled styling", () => {
    // "TEAM" in letter-spaced caps beside "About" and "Built-in capabilities" in
    // title case is what this looks like when it goes wrong — and it went wrong in
    // a component added the same day the type scale was swept.
    const offenders = [...tsxFiles()]
      .filter((f) => /uppercase tracking-wide text-(faint|muted)/.test(readFileSync(f, "utf8")))
      .map((f) => basename(f));
    expect(offenders, `use SEC_H for section headings:\n${offenders.join("\n")}`).toEqual([]);
  });
});
