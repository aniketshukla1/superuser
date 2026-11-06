import path from "node:path";
import { mkdirSync, rmSync } from "node:fs";
import type { Page } from "@playwright/test";
import { test } from "../e2e/fixtures";
import { architecture } from "./readmeArt/architecture";
import { hero, lightAt, lightUp } from "./readmeArt/hero";
import type { Theme } from "./readmeArt/palette";
import { BUTTONS, button, stats } from "./readmeArt/pieces";

// The README's drawn pieces: the moving hero, its buttons, the numbers strip, the architecture
// and the social card, each for GitHub's dark and light themes, in the app's own type. They land
// in docs/assets/raw (or DEMO_OUT); scripts/readme-assets.py turns them into docs/assets.
//
//   node node_modules/@playwright/test/cli.js test -c playwright.demo.config.ts readmeArt
//
// HERO_FRAMES=1 draws one still of the hero per theme, to judge the look before the full loop.
const OUT = process.env.DEMO_OUT || path.resolve(process.cwd(), "..", "..", "docs", "assets", "raw");
const FRAMES = Number(process.env.HERO_FRAMES || 72);
const THEMES: Theme[] = ["dark", "light"];

async function put(page: Page, html: string) {
  await page.evaluate((h) => {
    document.documentElement.style.background = "transparent";
    document.body.style.cssText = "margin:0;padding:0;background:transparent";
    document.body.innerHTML = h;
  }, html);
  await page.evaluate(() => document.fonts.ready);
}

const snap = (page: Page, file: string) =>
  page.locator("#card").screenshot({ path: path.join(OUT, file), omitBackground: true });

test("README art", async ({ page }) => {
  test.setTimeout(900_000);
  // A loop is only whole if every frame is from this run.
  rmSync(path.join(OUT, "hero"), { recursive: true, force: true });
  mkdirSync(path.join(OUT, "hero"), { recursive: true });
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto("/");
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });

  for (const theme of THEMES) {
    await put(page, hero(theme, 1280, 400));
    await lightUp(page, theme);
    for (let i = 0; i < FRAMES; i++) {
      await lightAt(page, i / FRAMES);
      await snap(page, `hero/hero-${theme}-${String(i).padStart(3, "0")}.png`);
    }
    await put(page, stats(theme));
    await snap(page, `stats-${theme}.png`);
    await put(page, architecture(theme));
    await snap(page, `architecture-${theme}.png`);
  }

  for (const b of BUTTONS) {
    await put(page, button(b));
    await snap(page, `${b.file}.png`);
  }
  await put(page, hero("dark", 1280, 640, true));
  await lightUp(page, "dark");
  await lightAt(page, 0.15);
  await snap(page, "social-preview.png");
});
