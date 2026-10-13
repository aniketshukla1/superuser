// The sidebar bottom is exactly ONE row — the account anchor (UX-DECISIONS §26).
// Contract under test: no "Settings & more", no standalone Inbox/Connectors rows; the
// inbox chip is state-driven (accent + count when pending) and clicks STRAIGHT to Inbox
// while the rest of the row opens the account menu, which always lists Inbox + Connectors.
import { expect } from "@playwright/test";
import { test } from "./fixtures";

test("the bottom is one account row — the old rows are gone", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("account-row")).toBeVisible();
  await expect(page.getByRole("button", { name: /Settings & more/i })).toHaveCount(0);
  // No standalone sidebar Inbox row: outside the menu, "Inbox" exists only as the chip.
  await expect(page.locator(".sidebar").getByRole("button", { name: "Inbox", exact: true })).toHaveCount(0);
});

test("pending items: the chip carries the count and goes straight to Inbox — no menu", async ({
  page,
}) => {
  await page.goto("/");
  const chip = page.getByTestId("inbox-chip");
  await expect(chip).toContainText(/\d/); // fixtures seed pending attention → accent count
  await chip.click();
  await expect(page.getByTestId("account-menu")).toHaveCount(0); // the chip never opens the menu
  // Inbox opened directly. The seeded "Approve: run_shell" item reads as the command it runs.
  await expect(page.getByRole("heading", { level: 1, name: "Inbox" })).toBeVisible();
  await expect(page.getByText("rm -rf build/")).toBeVisible();
});

test("the account menu: Inbox + Connectors always listed; Settings carries the shortcut hint", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  const menu = page.getByTestId("account-menu");
  await expect(menu.getByRole("menuitem", { name: "Inbox" })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: "Connectors", exact: true })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: /Settings/ })).toContainText("⌘");
  await expect(menu.getByRole("menuitem", { name: "Automations", exact: true })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: "Privacy", exact: true })).toBeVisible();
});

test("Settings is one click from the rail, and the inbox chip is its own control", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("settings-button").click();
  await expect(page.getByRole("heading", { level: 1, name: "Settings" })).toBeVisible();
  await expect(page.getByTestId("account-menu")).toHaveCount(0);
  // Not nested inside the account button: Tab reaches it, and it is announced on its own.
  await expect(page.locator('[data-testid="account-row"] [data-testid="inbox-chip"]')).toHaveCount(0);
});

test("Privacy in the menu is the audit log; Unrouted lives under Inbox ▸ Configure", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Privacy", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Privacy" })).toBeVisible();

  // §28: Messaging routing left the Connectors sub-nav entirely (Connectors · MCP only)…
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Connectors", exact: true }).click();
  await expect(page.getByRole("button", { name: "MCP servers" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Messaging routing/ })).toHaveCount(0);
  // The old fourth sub-nav tab is gone — exactly one page is named Privacy now
  // (renamed from Activity in f428ea1; the `audit` surface id is unchanged).
  await expect(page.getByRole("button", { name: "Privacy", exact: true })).toHaveCount(0);

  // …and Unrouted rides the Inbox's Configure tab.
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Inbox" }).click();
  await page.getByTestId("inbox-tab-configure").click();
  await expect(page.getByTestId("unrouted-section")).toBeVisible();
});
