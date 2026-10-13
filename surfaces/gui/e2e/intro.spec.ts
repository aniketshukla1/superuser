// The launch film (components/LaunchFilm): the whole film on a first launch, a short cut after
// that, and a way out of either at any moment. The fixtures turn the reel off for every
// other spec; these turn it back on.
import { expect, type Page } from "@playwright/test";
import { test } from "./fixtures";

/** Runs after the fixture's own init script, so this one has the last word. */
async function introOn(page: Page) {
  await page.addInitScript(() => {
    try {
      localStorage.removeItem("superuser.intro");
    } catch {
      /* nothing to undo */
    }
  });
}

test("a first launch plays the full reel, and Skip goes straight to the app", async ({ page }) => {
  await introOn(page);
  await page.goto("/");
  await expect(page.locator(".boot-term.reel-full")).toBeVisible();
  // The screens of work are flying, and the machine is locking onto them within a couple
  // of seconds — the film is playing, not stuck on a frame.
  await expect(page.locator(".film-feed")).toHaveCount(30);
  await expect(page.locator(".film-feed .film-lock").first()).toBeAttached({ timeout: 4_000 });
  const skip = page.getByTestId("intro-skip");
  await expect(skip).toBeVisible();
  await skip.click();
  // Skipping asks for the app now: the reel is gone well before its twelve seconds.
  await expect(page.locator(".boot-splash")).toHaveCount(0, { timeout: 2_500 });
  await expect(page.getByRole("button", { name: "New session", exact: true })).toBeVisible();
});

test("Escape skips too, and the next launch plays the short cut", async ({ page }) => {
  await introOn(page);
  await page.goto("/");
  await expect(page.locator(".boot-term.reel-full")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".boot-splash")).toHaveCount(0, { timeout: 2_500 });

  // Seen once: from now on the reel is the five-second cut.
  await page.reload();
  await expect(page.locator(".boot-term.reel-short")).toBeVisible();
});

test("the bottom readings are the real ones while the film plays", async ({ page }) => {
  await introOn(page);
  // Hold the runtime's answer: the film runs, and the runtime reading must not claim ok.
  await page.route("**/v1/health", async (route) => {
    await new Promise((r) => setTimeout(r, 2_500));
    await route.fallback();
  });
  await page.goto("/");
  const runtime = page.locator(".hud-steps .hs").first();
  await expect(runtime).toContainText("Local runtime");
  await expect(runtime).not.toHaveClass(/\bon\b/);
  await expect(runtime).toHaveClass(/\bon\b/, { timeout: 6_000 });
});
