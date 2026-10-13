import type { Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

export const SHOTS = path.resolve(process.cwd(), "ux-shots");

/** Where the app can be, as data — surface label → hash, plus how to tell it arrived. */
export const SURFACES = [
  { name: "session", hash: "#/", ready: "textarea" },
  { name: "brain", hash: "#/brain", ready: "main, .brain, [data-testid=brain-view]" },
  { name: "automations", hash: "#/automations", ready: "main, .watches, .auto" },
  { name: "inbox", hash: "#/inbox", ready: "main, .inbox" },
  { name: "connectors", hash: "#/connectors", ready: "main, .conn" },
  { name: "privacy", hash: "#/activity", ready: "main, .audit" },
  { name: "settings", hash: "#/settings", ready: "main, .set" },
] as const;

/** Settings sub-tabs, read from SettingsView's own tab bar at runtime. */
// The sizes that actually exist for this product, taken from `tauri.conf.json`:
// the shell opens at 1100×760 and lets the user drag down to 880×600. "min" is
// therefore a supported layout, not a hypothetical one.
export const VIEWPORTS = [
  { name: "wide", width: 1512, height: 945 },
  { name: "laptop", width: 1280, height: 800 },
  { name: "default", width: 1100, height: 760 },
  { name: "min", width: 880, height: 600 },
] as const;

export async function shot(page: Page, name: string) {
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.screenshot({ path: path.join(SHOTS, `${name}.png`), fullPage: false });
}

/** Navigate to a surface and wait for the shell to settle. */
export async function go(page: Page, hash: string) {
  const url = `/${hash}`;
  if (page.url().includes("localhost")) {
    await page.evaluate((h) => {
      window.location.hash = h.replace(/^#/, "#");
    }, hash);
  } else {
    await page.goto(url);
  }
  await page.waitForTimeout(450);
}

/** Boot the app on a surface, past the boot sequence. */
export async function boot(page: Page, hash = "#/") {
  await page.goto(`/${hash}`);
  // The launch film paints first; the sidebar is the shell's steady state.
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForTimeout(700);
}

/** Console errors + page errors + failed requests, collected for a whole test. */
export function watchErrors(page: Page) {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(`console.error: ${m.text()}`);
    if (m.type() === "warning" && /React|Warning:/i.test(m.text())) errors.push(`console.warn: ${m.text()}`);
  });
  page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
  page.on("requestfailed", (r) => {
    const u = r.url();
    // Vite HMR ping and favicon noise are not product failures.
    if (/__vite|favicon|\/@vite/.test(u)) return;
    errors.push(`requestfailed: ${r.method()} ${u} — ${r.failure()?.errorText}`);
  });
  return errors;
}
