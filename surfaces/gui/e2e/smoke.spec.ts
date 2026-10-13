import { test, expect } from "./fixtures";

test("app loads with the persona nav and composer", async ({ page }) => {
  await page.goto("/");
  // The window carries the product's name. The in-app wordmark is the boot splash's alone
  // since the sidebar's went (2026-08-24) — boot.spec.ts covers it; asserting it here only
  // passed when the check happened to catch the splash.
  await expect(page).toHaveTitle("Superuser");
  // New session tops the rail. Exact: each persona group's "+" is "New session with <name>".
  await expect(page.getByRole("button", { name: "New session", exact: true })).toBeVisible();
  // The persona groups render from /v1/personas, each under its short name.
  await expect(page.locator(".sidebar").getByText("Ops", { exact: true })).toBeVisible();
  await expect(page.getByTestId("composer-input")).toBeVisible();
});
