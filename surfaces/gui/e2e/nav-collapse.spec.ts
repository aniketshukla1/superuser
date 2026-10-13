// Left-nav polish (§20): collapse (⌘B / brand button → reveal button docks it back) and the
// RECENT-header group/filter popover (Group by Persona↔Chronological, Filter by coworker).
import { expect } from "@playwright/test";
import { test } from "./fixtures";

test("collapse hides the sidebar and reclaims the width; reveal button docks it back", async ({
  page,
}) => {
  await page.goto("/");
  const app = page.locator(".app");
  await expect(page.locator(".sidebar")).toBeVisible();

  // Collapse via the brand button.
  await page.getByRole("button", { name: "Collapse sidebar" }).click();
  await expect(app).toHaveClass(/nav-collapsed/);
  // The floating reveal affordance appears; clicking it docks the nav back.
  const reveal = page.getByRole("button", { name: "Show sidebar" });
  await expect(reveal).toBeVisible();
  await reveal.click();
  await expect(app).not.toHaveClass(/nav-collapsed/);
});

test("⌘B toggles the sidebar collapse", async ({ page }) => {
  await page.goto("/");
  const app = page.locator(".app");
  // The ⌘B listener is attached in an effect, so it doesn't exist until the app
  // has mounted. Pressing the key on a bare `goto` raced that and lost roughly one
  // run in five. Waiting on a rendered control is the honest ready signal — a real
  // user can't hit the shortcut before the window has painted either.
  await expect(page.getByTestId("account-row")).toBeVisible();
  await page.keyboard.press("Meta+b");
  await expect(app).toHaveClass(/nav-collapsed/);
  await page.keyboard.press("Meta+b");
  await expect(app).not.toHaveClass(/nav-collapsed/);
});

test("RECENT header group/filter popover: switch grouping + see assistant filters", async ({
  page,
}) => {
  await page.goto("/");
  const header = page.getByTestId("recent-header");
  await expect(header).toContainText("Recent");

  await header.getByRole("button", { name: "Group and filter conversations" }).click();
  const menu = page.getByTestId("group-filter-menu");
  await expect(menu).toContainText("Group by");
  // "Coworker" became "assistant" across the product (2026-08-28).
  await expect(menu).toContainText("Filter by assistant");

  // Switch to Chronological → the persona accordion collapses into a flat list (the
  // "Assistant" persona group header is no longer a row; sessions list directly).
  await menu.getByText("Chronological").click();
  await expect(menu.getByText("Chronological").locator("xpath=..")).toContainText("✓");

  // Filter-by-assistant checkboxes are present (none checked by default → all shown).
  await expect(menu).toContainText("None checked shows all.");
});
