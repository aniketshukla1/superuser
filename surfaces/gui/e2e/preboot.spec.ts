import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test, expect } from "./fixtures";

// The pre-boot colours mirror the base themes' --paper (Porcelain light, Carbon dark), so the
// expected values are read from the stylesheet rather than copied here to go stale. (Inlined
// rather than src/testing/source.ts: that one leans on vitest's __dirname, and this suite is ESM.)
const STYLES = join(dirname(fileURLToPath(import.meta.url)), "..", "src", "styles.css");
const inline = (file: string): string =>
  readFileSync(file, "utf8").replace(/^@import "(\.[^"]+\.css)";$/gm, (_, rel: string) => inline(join(dirname(file), rel)));
const CSS = inline(STYLES);
function paperOf(header: string): string {
  const block = CSS.slice(CSS.indexOf(header), CSS.indexOf("\n}", CSS.indexOf(header)));
  const hex = block.match(/--paper:\s*#([0-9a-fA-F]{6})/)![1];
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16));
  return `rgb(${r}, ${g}, ${b})`;
}
const LIGHT_PAPER = paperOf(':root, .theme-preview[data-palette="porcelain"] {');
const DARK_PAPER = paperOf('html[data-theme="dark"], .theme-preview[data-palette="carbon"] {');

// The window before the app: what index.html paints on its own, before main.tsx has run.
// It used to be nothing — an empty #root on the webview's blank page — so a dev server still
// compiling, a dead one, or a half-saved file that broke an import all looked identical: a
// blank window, with the reason only in a terminal.

test("a normal start replaces the pre-boot screen with the app", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("composer-input")).toBeVisible();
  await expect(page.locator(".preboot")).toHaveCount(0);
});

test("a module that fails to load says so instead of leaving a blank window", async ({ page }) => {
  // The shape of the real failure: a file saved mid-edit, so the import graph throws.
  await page.route("**/src/main.tsx", (r) =>
    r.fulfill({ contentType: "application/javascript", body: "export const half = ;" }),
  );
  await page.goto("/");
  await expect(page.locator(".preboot-mark")).toHaveText("superuser");
  await expect(page.locator("#preboot-note")).toContainText("The interface didn't start");
  // Painted in the app's own ground, not the webview's default white — Carbon's, the default.
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(bg).toBe(DARK_PAPER);
});

test("a start that never finishes says it is still starting", async ({ page }) => {
  // A dev server that accepted the request and never answered: no error ever fires.
  await page.route("**/src/main.tsx", () => new Promise(() => {}));
  // A module script that never arrives holds DOMContentLoaded too — the page is painted long
  // before that event, so wait only for the navigation to commit.
  await page.goto("/", { waitUntil: "commit" });
  await expect(page.locator(".preboot-mark")).toBeVisible();
  await expect(page.locator("#preboot-note")).toContainText("Still starting", { timeout: 15_000 });
});

// "dark" is the value Settings stored before there were themes; it still means a dark start.
test("dark users get a dark pre-boot screen, not a white flash", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("superuser.theme", "dark"));
  await page.route("**/src/main.tsx", () => new Promise(() => {}));
  // A module script that never arrives holds DOMContentLoaded too — the page is painted long
  // before that event, so wait only for the navigation to commit.
  await page.goto("/", { waitUntil: "commit" });
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(bg).toBe(DARK_PAPER);
});
