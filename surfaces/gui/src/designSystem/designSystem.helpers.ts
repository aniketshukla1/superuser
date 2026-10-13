import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { readStyles } from "../testing/source";

// The scales, checked against the source rather than trusted to a convention.
//
// The state this replaced: eleven radii (2,3,4,5,6,7,8,9,10,12,16), eighteen type
// sizes, and fourteen button padding pairs doing one job. None of that was
// carelessness — each was a reasonable local decision made with nowhere to look the
// answer up, which is exactly the failure a convention cannot fix and a test can.
//
// A test rather than a lint rule because the failure mode is invisible: a wrong
// radius, a half-pixel line-height, a button 7px too tall — all of it renders
// perfectly, and only an eye catches it. The owner's eye caught all three.

export const SRC = join(__dirname, "..");
export const CSS = readStyles();
export const CONFIG = readFileSync(join(SRC, "..", "tailwind.config.js"), "utf8");

export const px = (name: string): number => {
  const m = CSS.match(new RegExp(`\\${name}:\\s*(\\d+(?:\\.\\d+)?)px`));
  if (!m) throw new Error(`${name} is not defined in styles.css`);
  return Number(m[1]);
};

/** Every .ts under src/ — shared class-name constants live in these. */
export function tsFiles(dir = SRC): string[] {
  return readdirSync(dir).flatMap((e) => {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) return tsFiles(p);
    return p.endsWith(".ts") && !p.endsWith(".test.ts") && !p.endsWith(".d.ts") ? [p] : [];
  });
}

/** Every .tsx under src/, so a sweep can't miss a folder. */
export function tsxFiles(dir = SRC): string[] {
  return readdirSync(dir).flatMap((e) => {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) return tsxFiles(p);
    return p.endsWith(".tsx") && !p.endsWith(".test.tsx") ? [p] : [];
  });
}
// Radius got a rule and the rest did not, so a whole panel could sit on the scale
// for corners and hand-write every padding, gap and size inside it — which is
// exactly what the stats panel did: seven off-scale font sizes (12.5, 11.5, 14)
// and paddings off the 4px grid. Checked per component block, because that is the
// unit a person writes: someone styling one panel should not be able to invent a
// private spacing system inside it.
export const SPACED = ["wstats", "askcard", "sched-run", "nav-icon-btn", "navrow"];
