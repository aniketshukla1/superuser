// For tests that read this app's own source as text.
//
// A module split into a folder (`App.tsx` + `App/*.tsx`, or Rust's `personas.rs` +
// `personas/*.rs`) is no longer one file. `readModule` reads the file and every
// non-test file in its folder, so a search over "App.tsx" still sees all of App.
//
// styles.css is a list of `@import`s, one file per section, so reading it alone
// gives the list and no rules. `readStyles` follows the imports in order and
// returns the stylesheet the app actually loads.
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { basename, dirname, extname, join } from "node:path";

const SRC = join(__dirname, "..");

function inline(file: string): string {
  const text = readFileSync(file, "utf8");
  return text.replace(/^@import "(\.[^"]+\.css)";$/gm, (_, rel: string) => inline(join(dirname(file), rel)));
}

/** The whole stylesheet, imports inlined in cascade order. */
export const readStyles = (): string => inline(join(SRC, "styles.css"));

const isTest = (f: string) => /\.(test|spec)\.[jt]sx?$/.test(f) || /^(tests|\w+_tests)(\.rs)?$/.test(basename(f));

function walk(dir: string): string[] {
  return readdirSync(dir)
    .sort()
    .flatMap((e) => {
      const p = join(dir, e);
      if (isTest(p)) return [];
      return statSync(p).isDirectory() ? walk(p) : [p];
    });
}

/** `file` plus everything in the folder of the same name beside it. */
export function readModule(file: string): string {
  const folder = join(dirname(file), basename(file, extname(file)));
  const parts = existsSync(folder) && statSync(folder).isDirectory() ? walk(folder) : [];
  return [...parts, file].map((f) => readFileSync(f, "utf8")).join("\n");
}
