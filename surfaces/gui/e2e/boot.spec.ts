// Cold-boot fixes (owner-hit 2026-07-23): the splash identifies as this product rather
// than wearing the ✦ text glyph that read as another product's logo, and the model picker
// recovers when the mount-time settings fetch loses the race against the sidecar boot —
// previously "Loading models…" stuck until the user visited Settings.
//
// The splash became a phosphor-terminal readout in f636176 (the Superuser rebrand), which
// retired the `.boot-mark` star this spec used to assert. The guard that still matters is
// the same one: it says superuser, and it never shows the sparkle.
import { expect } from "@playwright/test";
import { test } from "./fixtures";

test("boot splash identifies as superuser, not the sparkle glyph", async ({ page }) => {
  // Hold health long enough to observe the splash.
  await page.route("**/v1/health", async (route) => {
    await new Promise((r) => setTimeout(r, 1500));
    await route.fallback();
  });
  await page.goto("/");
  const term = page.locator(".boot-term");
  await expect(term).toBeVisible();
  // Names the product, and never the glyph that belonged to another one.
  await expect(term.locator(".boot-head")).toContainText("superuser");
  await expect(term).not.toContainText("✦");
  // "Starting Superuser" is the terminal's accessible name, not visible text.
  await expect(page.getByRole("status", { name: /Starting Superuser/ })).toBeVisible();
});

test("model picker recovers when settings fetches die during sidecar boot", async ({ page }) => {
  // Real cold-start shape: EVERY request fails until the sidecar is up (health included),
  // then everything answers. The mount-time settings fetches all lose that race and are
  // swallowed — the post-health reload must populate the picker without a Settings visit.
  let sidecarUp = false;
  await page.route("**/v1/health", async (route) => {
    await new Promise((r) => setTimeout(r, 700));
    sidecarUp = true;
    await route.fallback();
  });
  await page.route("**/v1/settings", async (route) => {
    if (route.request().method() === "GET" && !sidecarUp) {
      await route.abort();
      return;
    }
    await route.fallback();
  });
  await page.goto("/");
  // The picker is the composer's Model chip (the `.dd` dropdown it replaced went with the
  // 2026-08-26 composer rebuild); it carries the short name, the menu the full label.
  await expect(page.getByTestId("model-chip")).toContainText("Claude Opus 4.8", {
    timeout: 10_000,
  });
  await expect(page.getByTestId("models-loading")).toHaveCount(0);
});
