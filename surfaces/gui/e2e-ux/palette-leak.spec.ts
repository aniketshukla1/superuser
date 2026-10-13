import { test, expect, boot, shot } from "./audit";

// The concrete consequence of the palette having no focus trap: keyboard input
// reaches the app *behind* the scrim while the overlay is still covering it.

test("typing while the palette is open cannot reach the composer behind it", async ({ page }) => {
  await boot(page);
  const search = page.getByRole("button", { name: /search/i }).first();
  if (await search.count()) await search.click();
  else await page.keyboard.press("Meta+k");
  await page.waitForTimeout(300);
  await expect(page.getByPlaceholder(/search chats/i)).toBeVisible();

  // Tab past the palette's own rows, into the page underneath.
  for (let i = 0; i < 15; i++) await page.keyboard.press("Tab");

  const landedOnComposer = await page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    return el?.tagName.toLowerCase() === "textarea";
  });

  if (landedOnComposer) {
    await page.keyboard.type("this went into the composer under the overlay");
    await shot(page, "palette-focus-leak");
    const composerText = await page.locator("textarea").first().inputValue();
    const paletteStillOpen = await page.getByPlaceholder(/search chats/i).count();
    expect(
      { composerText, paletteStillOpen },
      "keystrokes landed in the composer while the search overlay was still covering it",
    ).toEqual({ composerText: "", paletteStillOpen: 1 });
  }
  expect(landedOnComposer, "Tab reached the composer behind the open palette").toBe(false);
});
