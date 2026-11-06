import path from "node:path";
import { mkdirSync } from "node:fs";
import type { Page } from "@playwright/test";
import { test } from "../e2e/fixtures";
import { EGRESS_LOCAL_ONLY, HERO } from "./demoData";
import { dress, serve } from "./dress";

// The README's images, taken from the real UI over the hermetic mock with the demo data laid
// on top. `DEMO_OUT` overrides where they land (default: docs/assets/raw at the repo root);
// scripts/readme-assets.py turns the raw frames into the GIF and the sized screenshots. The
// banner, buttons, numbers and diagram are drawn by readmeArt.spec.ts.
const SIZE = { width: 1440, height: 900 };
const OUT = process.env.DEMO_OUT || path.resolve(process.cwd(), "..", "..", "docs", "assets", "raw");

let frame = 0;
/** A clean screenshot for the README's grid and, with a caption, a numbered frame for the GIF. */
async function shot(page: Page, name: string, caption?: string) {
  mkdirSync(OUT, { recursive: true });
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(OUT, `${name}.png`) });
  if (!caption) return;
  await page.evaluate((text) => {
    const el = document.createElement("div");
    el.id = "demo-caption";
    el.textContent = text;
    el.style.cssText =
      "position:fixed;left:50%;bottom:30px;transform:translateX(-50%);z-index:99999;max-width:82%;" +
      "padding:13px 24px;border-radius:12px;background:rgba(18,21,23,.97);border:1px solid #30363c;" +
      "color:#e8eaed;font:500 19px/26px Inter,system-ui,sans-serif;text-align:center;" +
      "box-shadow:0 14px 44px rgba(0,0,0,.55)";
    document.body.appendChild(el);
  }, caption);
  frame += 1;
  await page.screenshot({ path: path.join(OUT, `frame-${String(frame).padStart(2, "0")}-${name}.png`) });
  await page.evaluate(() => document.getElementById("demo-caption")?.remove());
}

async function go(page: Page, hash: string, wait = 900) {
  await page.evaluate((h) => { location.hash = h; }, hash);
  await page.waitForTimeout(wait);
}

test("README screenshots", async ({ page }) => {
  await page.setViewportSize(SIZE);
  await page.addInitScript(() => localStorage.setItem("superuser.theme", "dark"));
  await page.goto("/");
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await dress(page);
  await page.reload();
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });

  await go(page, `#/chat/${HERO}`, 1500);
  // The conversation is the subject; the side panel has nothing to show for a replayed one.
  const hide = page.getByLabel("Hide side panel");
  if (await hide.count()) await hide.first().click();
  await shot(page, "chat", "Ask an assistant. It shows its work: the steps, a chart, facts apart from guesses.");

  await go(page, "#/inbox");
  await shot(page, "inbox", "Anything that leaves your computer waits for your yes.");

  await go(page, "#/settings?tab=boundaries", 1200);
  await shot(page, "boundaries", "Boundaries: rules you can read, checked before every action.");

  await go(page, "#/activity");
  await shot(page, "network", "Privacy: everything that left this machine, and who received it.");

  await go(page, "#/brain", 1200);
  await shot(page, "brain", "What it knows about you is markdown files on your disk.");

  await go(page, "#/connectors", 1200);
  await shot(page, "connectors");
  await page.getByText("Prometheus", { exact: true }).first().click();
  await shot(page, "connector-prometheus", "Connect your own tools, and see what assistants can do with each.");

  await go(page, "#/automations", 1200);
  await shot(page, "automations", "Routines run while you are away, and report back.");
});

test("README screenshot: local-only refuses", async ({ page }) => {
  await page.setViewportSize(SIZE);
  await page.addInitScript(() => localStorage.setItem("superuser.theme", "dark"));
  await page.goto("/");
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await dress(page);
  await serve(page, "/v1/egress", EGRESS_LOCAL_ONLY);
  await page.reload();
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await go(page, "#/activity", 1200);
  frame = 90; // after the tour's frames, whatever order the two tests ran in
  await shot(page, "network-local-only", "Local-only mode: nothing leaves, and every refusal is on the record.");
});
