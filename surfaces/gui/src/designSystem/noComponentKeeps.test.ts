import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { describe, expect, it } from "vitest";
import { readModule } from "../testing/source";
import { CSS, SRC, tsxFiles } from "./designSystem.helpers";


describe("no component keeps its own numbers", () => {

  // The 26px row height and the 12px name are set in two different rules, and the
  // name's rule is the one that can break the other: --t-sm normally pairs with a
  // 16px leading, and taking that pairing shrinks every sidebar row to 24px. This
  // caught it once already.
  it("a row name takes its leading from the row, not from --t-sm-lh", () => {
    const rule = CSS.match(/\.sidebar \.list-row \.row-name \{([^}]*)\}/);
    expect(rule, ".row-name rule is missing").toBeTruthy();
    const body = rule![1];
    expect(body).toMatch(/font-size:\s*var\(--t-sm\)/);
    expect(body, "line-height must inherit the row's, not re-declare one").toMatch(
      /line-height:\s*inherit/,
    );
    // And the row must still be the one declaring it.
    expect(CSS).toMatch(/\.sidebar \.list-row \{[^}]*line-height:\s*18px/);
  });

  // A fade that is always on is not an affordance, it is a gradient. This asserts the
  // mask hangs off the state class and never off the bare element.
  it("the user-prompt fade is on the clamped state, not on every prompt", () => {
    const base = CSS.match(/\.user-clamp \{([^}]*)\}/);
    expect(base, ".user-clamp rule is missing").toBeTruthy();
    expect(base![1], "the fade must not be on the base rule").not.toMatch(/mask-image/);
    expect(CSS).toMatch(/\.user-clamp\.is-clamped \{[^}]*mask-image:\s*linear-gradient/);
  });

  // A control you can only find by hovering the thing it sits on is a control most
  // people never find, and one nobody on a touch or keyboard-only path finds at all.
  // This one was hover-gated for a while; --faint already keeps it quiet.
  it("the per-row + is present at rest, not revealed on hover", () => {
    const rule = CSS.match(/\.nav-row-plus \{([^}]*)\}/);
    expect(rule, ".nav-row-plus rule is missing").toBeTruthy();
    expect(rule![1], "no opacity gate on the resting state").not.toMatch(/opacity/);
    expect(CSS, "nothing may reveal it on hover").not.toMatch(/:hover[^{]*\.nav-row-plus/);
    // And it stays pulled into the chevron: the trailing pair is one control.
    expect(rule![1]).toMatch(/margin-right:\s*calc\(var\(--s-2\) \* -1\)/);
  });

  // Two controls were measured under the 24px a pointer target has to be: the
  // transcript rail's marks at 32x12, and the prompt's "Show more" at 64x18. Both are
  // small by design and both are still small; what changed is that their BOX now
  // reaches the control floor. A literal here would be someone shrinking them back.
  it("small controls keep the hit-target floor, from the token", () => {
    for (const sel of ["\\.tnav-item", "\\.user-clamp-more"]) {
      const rule = CSS.match(new RegExp(`${sel} \\{([^}]*)\\}`));
      expect(rule, `${sel} rule is missing`).toBeTruthy();
      expect(rule![1], `${sel} must floor at --ctl-h-sm`).toMatch(
        /min-height:\s*var\(--ctl-h-sm\)/,
      );
    }
  });

  // The app rendered 19 distinct text sizes against a five-step scale, and the most
  // common one — 12.5px, 164 uses — was not a step. Half-steps do not read as a
  // decision, they read as nobody deciding. Display type (>=22px) is exempt: greetings
  // and boot marks are one-offs the scale has no step for and should not pretend to.
  it("every text size in a component is on the scale", () => {
    const scale = new Set(["11", "12", "13", "15", "19"]);
    const offenders: string[] = [];
    for (const f of tsxFiles()) {
      const src = readFileSync(f, "utf8");
      for (const m of src.matchAll(/text-\[([\d.]+)px\]/g)) {
        if (Number(m[1]) >= 22) continue;
        if (!scale.has(m[1])) offenders.push(`${basename(f)}: text-[${m[1]}px]`);
      }
    }
    expect(offenders, `off-scale sizes: ${offenders.slice(0, 8).join(", ")}`).toEqual([]);
  });

  // styles.css says the same thing in tokens, so a change to the scale reaches
  // everything it should. hiw-* and chart-* are drawings at their own scale, not UI
  // text, and rounding them to a UI scale would simply be wrong.
  it("stylesheet font sizes come from the tokens, not from numbers", () => {
    const offenders: string[] = [];
    let sel = "";
    for (const line of CSS.split("\n")) {
      const t = line.trim();
      if (line.includes("{") && !t.startsWith("/*") && !t.startsWith("*")) {
        sel = line.split("{")[0].trim() || sel;
      }
      if (sel.includes(".hiw") || sel.includes(".chart")) continue;
      for (const m of line.matchAll(/font-size:\s*([\d.]+)px/g)) {
        // This loop used to end with `if (font-size >= 22) continue`, which exempted
        // precisely the sizes that shout — a 22px mark over a 24px heading sat in the
        // folder gate, the dialog that opens in front of everything else, and the
        // guard waved both through for being big enough. The scale tops out at --t-xl
        // (19px), so anything above it is off the scale BY DEFINITION.
        //
        // There is no exemption now. What was above the scale was a display tier the
        // tokens never had — a hero greeting, an onboarding heading, a boot splash,
        // each at its own invented number — and rather than mint tokens for a tier
        // nobody had agreed on, they came down onto --t-xl. So the rule is simply
        // that the scale IS the sizes, with no "unless it is large" attached.
        offenders.push(`${sel}: ${m[1]}px`);
      }
    }
    expect(offenders, `bare px sizes: ${offenders.slice(0, 8).join(", ")}`).toEqual([]);
  });

  // Thirty-five button and chip treatments, five on the tokens: nine distinct heights
  // and twelve distinct padding pairs for controls doing the same job. That is where
  // "some buttons have too much padding" comes from — not one bad rule, thirty rules
  // that never agreed. Icons and thumbnails INSIDE a chip are exempt: they are
  // decoration, not the control, and sizing a glyph to a button height is not
  // consistency.
  it("button and chip sizes come from the control tokens", () => {
    const offenders: string[] = [];
    for (const m of CSS.matchAll(/(^|\n)([^\n{}]+)\{([^}]*)\}/g)) {
      const sel = m[2].trim();
      if (!/btn|chip|pill|\bctl\b/.test(sel)) continue;
      if (/:(hover|focus|active|disabled|not)/.test(sel)) continue;
      if (sel.includes(".hiw")) continue; // miniature illustrations, their own scale
      if (/\b(ico|icon|img)\b|-ico\b|-icon\b/.test(sel)) continue; // decoration inside a control
      for (const prop of ["height", "padding"]) {
        const v = new RegExp(`(?<![-\\w])${prop}:\\s*([^;]+)`).exec(m[3])?.[1]?.trim();
        if (!v || v === "0" || v.includes("var(")) continue;
        offenders.push(`${sel}: ${prop}: ${v}`);
      }
    }
    expect(offenders, `hand-sized controls: ${offenders.slice(0, 6).join(" | ")}`).toEqual([]);
  });

  // Eleven icon sizes across 136 call sites, and 13/14/15/16 accounted for 114 of them —
  // four adjacent values doing one job, because each was picked next to whatever it sat
  // beside. FilesTab was the tell: folder rows at 22 and file rows at 20, in the same
  // list. `size` takes a number, so the scale is a convention; this is what makes it
  // a rule.
  it("icons are sized from ICON, not from a number", () => {
    const offenders: string[] = [];
    for (const f of tsxFiles()) {
      const src = readFileSync(f, "utf8");
      for (const m of src.matchAll(/<Icon\b(?:(?!\/>|>)[\s\S])*?size=\{(\d+(?:\.\d+)?)\}/g)) {
        offenders.push(`${basename(f)}: size={${m[1]}}`);
      }
    }
    expect(offenders, `hand-sized icons: ${offenders.slice(0, 6).join(", ")}`).toEqual([]);
  });

  // The glyph components are icons too, and they were the ones actually rendering an
  // off-scale size after the <Icon> sweep — a PersonaGlyph at 14 in a row of 15s.
  // ConnectorBadge is on the AVATAR scale instead: a 44px brand tile is not an icon that
  // grew, and it had nine sizes across fifteen files.
  it("glyph and badge components are sized from a scale too", () => {
    const offenders: string[] = [];
    for (const f of tsxFiles()) {
      const src = readFileSync(f, "utf8");
      for (const m of src.matchAll(/<(PersonaGlyph|ConnectorBadge)\b(?:(?!\/>|>)[\s\S])*?size=\{(\d+)\}/g)) {
        offenders.push(`${basename(f)}: ${m[1]} size={${m[2]}}`);
      }
    }
    expect(offenders, `hand-sized glyphs: ${offenders.slice(0, 6).join(", ")}`).toEqual([]);
  });

  // The scale is written twice — once in TS for the `size` prop, once in CSS for the
  // boxes that hold a glyph. Two languages, one scale: a box that holds a 15px glyph and
  // a glyph that renders at 15px must not be able to disagree about what 15 is.
  it("the CSS icon tokens match the TS ones", () => {
    const ts = readModule(join(SRC, "components", "Icon.tsx"));
    for (const [name, key] of [["--i-sm", "sm"], ["--i-md", "md"], ["--i-lg", "lg"], ["--i-xl", "xl"]]) {
      const css = new RegExp(`${name}:\\s*(\\d+)px`).exec(CSS)?.[1];
      const tsv = new RegExp(`\\b${key}:\\s*(\\d+),`).exec(ts)?.[1];
      expect(css, `${name} missing from styles.css`).toBeTruthy();
      expect(tsv, `ICON.${key} missing from Icon.tsx`).toBeTruthy();
      expect(css, `${name} (${css}) and ICON.${key} (${tsv}) disagree`).toBe(tsv);
    }
  });
});
