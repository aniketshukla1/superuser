import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { readModule } from "../testing/source";
import { CSS, SPACED, SRC, px, tsxFiles } from "./designSystem.helpers";


describe("no component keeps its own numbers", () => {

  // 632 spacing literals across 33 distinct values, including 113 odd pixels — 3, 5, 7,
  // 9, 11 — that nobody could have chosen on purpose twice. Rounded down: tighter is the
  // direction this UI has been going, and rounding down only ever removes a pixel of air
  // rather than adding one to a layout that was already snug.
  //
  // Three things stay literal, and each is a different reason:
  //   0 and 1px  a reset, and a hairline nudge for optical alignment — not spacing
  //   >= 20px    layout: page and section rhythm, which the component scale does not
  //              cover and should not pretend to
  //   hiw/chart  drawings at their own scale
  it("stylesheet spacing comes from the tokens, not from numbers", () => {
    const props = /(?<![-\w])((?:padding|margin|gap|row-gap|column-gap)(?:-(?:top|right|bottom|left))?):\s*([^;}]+)/g;
    const offenders: string[] = [];
    let sel = "";
    let inRoot = false;
    for (const line of CSS.split("\n")) {
      const t = line.trim();
      if (line.includes("{") && !t.startsWith("/*") && !t.startsWith("*")) {
        sel = line.split("{")[0].trim() || sel;
        inRoot = sel.startsWith(":root") || sel === "html";
      }
      if (inRoot || sel.includes(".hiw") || sel.includes(".chart")) continue;
      for (const m of line.matchAll(props)) {
        for (const part of m[2].split(/\s+/)) {
          const v = /^(\d+)px$/.exec(part);
          if (!v) continue;
          const n = Number(v[1]);
          if (n === 0 || n === 1 || n >= 20) continue;
          offenders.push(`${sel}: ${m[1]}: ${part}`);
        }
      }
    }
    expect(offenders, `hand-set spacing: ${offenders.slice(0, 6).join(" | ")}`).toEqual([]);
  });

  // The scale used to skip 10 and 14 while the app used them 262 times. It steps with
  // Tailwind's 0.5..4 now, so a component and the stylesheet cannot disagree about what
  // "one step" is.
  it("the spacing scale is every even step from 2 to 16", () => {
    const want = [2, 4, 6, 8, 10, 12, 14, 16];
    expect(want.map((_, i) => px(`--s-${i + 1}`))).toEqual(want);
  });

  // Arbitrary spacing in a component is how the stylesheet's odd pixels got in the first
  // time. Three survive, each a computed ALIGNMENT rather than a gap — a number derived
  // from an icon plus a gap so a label lines up under the one above it — and each is
  // listed here by name. Adding a fourth is then a decision someone has to make on
  // purpose, in this file, with a reason, instead of a number typed into a className.
  it("components do not invent their own spacing", () => {
    const ALIGNMENTS = new Set([
      "pl-[19px]", // session names under the folder NAME: folder icon + gap
      "ml-[15px]", // the same indent, one level out
      "mt-[1px]", // a hairline optical nudge, below the scale's floor
    ]);
    const offenders: string[] = [];
    for (const f of tsxFiles()) {
      const src = readFileSync(f, "utf8");
      for (const m of src.matchAll(
        /\b(?:p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr|gap|gap-x|gap-y|space-x|space-y)-\[[^\]]+\]/g,
      )) {
        if (!ALIGNMENTS.has(m[0])) offenders.push(`${basename(f)}: ${m[0]}`);
      }
    }
    expect(offenders, `invented spacing: ${offenders.slice(0, 6).join(", ")}`).toEqual([]);
  });

  // CHAT_MIN_W is enforced in TS; --chat-min exists so the stylesheet can reason about
  // the same number. Two places, one value — the failure mode of a mirror is that it
  // stops being one.
  it("the chat minimum is the same number in both languages", () => {
    const rh = readModule(join(SRC, "components", "ResizeHandle.tsx"));
    const ts = /CHAT_MIN_W\s*=\s*(\d+)/.exec(rh)?.[1];
    const css = /--chat-min:\s*(\d+)px/.exec(CSS)?.[1];
    expect(ts, "CHAT_MIN_W missing").toBeTruthy();
    expect(css, "--chat-min missing").toBeTruthy();
    expect(css, `--chat-min (${css}) and CHAT_MIN_W (${ts}) disagree`).toBe(ts);
  });

  it("uses tokens for spacing and type, not literals", () => {
    const offenders: string[] = [];
    for (const name of SPACED) {
      const rules = [...CSS.matchAll(new RegExp(`(?:^|\\n)(\\.${name}[^{}]*)\\{([^}]*)\\}`, "g"))];
      expect(rules.length, `no .${name} rules found — did the class get renamed?`).toBeGreaterThan(0);
      for (const [, sel, body] of rules) {
        for (const prop of ["padding", "gap", "font-size", "border-radius", "margin-bottom", "margin-top"]) {
          for (const [, value] of body.matchAll(new RegExp(`(?:^|;|\\s)${prop}:\\s*([^;]+)`, "g"))) {
            if (value.includes("var(")) continue;
            // 0, auto and `none` are not sizes.
            if (/^(0|auto|none|0 auto|inherit)$/.test(value.trim())) continue;
            offenders.push(`${sel.trim().split(",")[0]} { ${prop}: ${value.trim()} }`);
          }
        }
      }
    }
    expect(offenders, `hand-written values:\n${offenders.join("\n")}`).toEqual([]);
  });
});

describe("controls", () => {
  it("derives its height rather than choosing one", () => {
    // The complaint that started this: a 12.5px label in a 35px-tall button.
    // Nothing chose 35 — it fell out of an inherited line-height plus py-2.
    expect(px("--ctl-h")).toBe(px("--t-md-lh") + 2 * px("--ctl-pad-y"));
  });

  it("stays above the pointer-target floor", () => {
    // WCAG 2.5.8 wants 24×24 CSS px. The dense variant is the one at risk.
    expect(px("--ctl-h-sm")).toBeGreaterThanOrEqual(24);
  });

  it("never defines one control class at two specificities", () => {
    // The exact bug this is for. `.btn` was already a convention here — 31 call
    // sites, with .primary/.danger/.icon-only — and it is a COMPLETE button:
    // sizing plus an outlined skin. Adding a second, bare `.btn` for sizing alone
    // did not replace it, it LOST to it: `button.btn` is (0,1,1) and `.btn` is
    // (0,1,0). Every swept constant silently kept the old 35px box, and any
    // `bg-accent` on it lost to the skin's background too. Nothing errored.
    // It measured 35px in the running app, which is how it was found.
    for (const cls of ["btn", "ctl", "field", "btn-primary"]) {
      const bare = new RegExp(`(?:^|\n)\.${cls}[\s,{:]`).test(CSS);
      const qualified = new RegExp(`(?:^|\n)(?:button|input|a)\.${cls}[\s,{:.]`).test(CSS);
      expect(
        bare && qualified,
        `\`.${cls}\` is defined both bare and element-qualified — the bare rule always loses`,
      ).toBe(false);
    }
  });

  it("keeps every control's size in the recipe, not scattered across rules", () => {
    // A padding on a button outside the recipe is a second opinion about height,
    // and it is how fourteen different pairs accumulated in the first place.
    const strays = [...CSS.matchAll(/(?:^|\n)((?:button|\.ctl)[^{]*)\{([^}]*)\}/g)]
      .filter((m) => /\bpadding:\s*[^;]*\d+px/.test(m[2]))
      .map((m) => m[1].trim().replace(/\s+/g, " "));
    expect(strays, `these size a control outside the recipe:\n${strays.join("\n")}`).toEqual([]);
  });

  it("leaves no button padding in the shared style constants", () => {
    // These constants each dress many buttons, so a padding pair in one is a
    // height decided in a dozen places at once. The recipe is `.btn`.
    const offenders: string[] = [];
    for (const f of tsxFiles()) {
      const src = readFileSync(f, "utf8");
      for (const m of src.matchAll(/const (BTN|OPT|CHIP|TAB|PILL)[A-Z_]*\s*=\s*\n?\s*"([^"]*)"/g)) {
        if (/\bp[xy]-[0-9.]+ p[xy]-[0-9.]+\b/.test(m[2])) offenders.push(`${f.split("/").pop()}: ${m[0].slice(0, 60)}`);
      }
    }
    expect(offenders, `button padding outside .btn:\n${offenders.join("\n")}`).toEqual([]);
  });
});

describe("pages", () => {
  it("gives every surface with sections one tab style and one column", () => {
    // Six surfaces had four tab styles — a 208px side column, an underline under the lead,
    // pills above a page with no title, a boxed segment — and two column widths, so the title
    // moved sideways going from Inbox to Brain. PageTabs is the one tab row; the column is
    // max-w-4xl wherever it is used.
    // By module: a file inside a split component's folder (`WatchesView/…`) is that
    // component, so the check reads the whole module, not the one file.
    const moduleOf = (f: string) => (existsSync(`${dirname(f)}.tsx`) ? `${dirname(f)}.tsx` : f);
    const surfaces = [...new Set(tsxFiles().filter((f) => /<PageTabs\b/.test(readFileSync(f, "utf8"))).map(moduleOf))];
    expect(surfaces.map((f) => basename(f)).sort()).toEqual([
      "AuditView.tsx",
      "BrainView.tsx",
      "InboxView.tsx",
      "IntegrationsView.tsx",
      "SettingsView.tsx",
      "WatchesView.tsx",
    ]);
    for (const f of surfaces) {
      const src = readModule(f);
      expect(src, `${basename(f)} lays its page out on a different column`).toMatch(/max-w-4xl mx-auto px-7 py-6/);
      expect(src, `${basename(f)} keeps a hand-rolled tab row beside PageTabs`).not.toMatch(/aria-current=\{[^}]*"page"/);
    }
  });
});
