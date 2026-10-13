// Automations management — the parts of Rohit's manual pass that automations.spec.ts (run-banner +
// Back) doesn't cover: the task list, triggering a manual run (POST .../run appends a run and opens
// its live session), pausing via the enable toggle, and deleting. Seeded with one task.
import { expect } from "@playwright/test";
import { test } from "./fixtures";

async function openAutomations(page) {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Automations", exact: true }).click();
  await expect(page.getByText("Recurring tasks Superuser runs at a set time.")).toBeVisible();
}

test("lists a scheduled task with its schedule and run count", async ({ page }) => {
  await openAutomations(page);
  const card = page.locator(".sched-card", { hasText: "Daily AI News" });
  await expect(card).toBeVisible();
  // The server's "~5:40 PM" is said as a word and in the system clock.
  await expect(card).toContainText(/Every day at about 5:40/i);
  await expect(card).toContainText("running now");
});

test("Run now triggers a manual run and opens its live session", async ({ page }) => {
  await openAutomations(page);
  await page.locator(".sched-card", { hasText: "Daily AI News" }).click();
  await page.getByRole("button", { name: /Run now/ }).click();
  // The manual run opens as a session with the automation-context banner.
  const banner = page.getByTestId("run-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toContainText("Daily AI News");
});

test("enable toggle pauses the task", async ({ page }) => {
  await openAutomations(page);
  await page.locator(".sched-card", { hasText: "Daily AI News" }).click();
  // The on/off is a switch with its state written beside it.
  const toggle = page.getByTestId("task-enabled");
  await expect(toggle).toHaveAttribute("aria-checked", "true");
  await expect(page.locator(".sched-state-label")).toHaveText("Active");
  await toggle.click();
  await expect(page.locator(".sched-state-label")).toHaveText("Paused");
});

test("delete asks first, then removes the task; deleting the last one shows the empty state", async ({ page }) => {
  await openAutomations(page);
  await page.locator(".sched-card", { hasText: "Daily AI News" }).click();
  await page.getByRole("button", { name: /Delete/ }).click();
  // One click no longer deletes from the detail page — it asks, as the list card does.
  await page.getByTestId("task-delete-confirm").click();
  // Back on the list, the deleted task is gone; the other seeded task remains.
  await expect(page.locator(".sched-card", { hasText: "Daily AI News" })).toHaveCount(0);
  await expect(page.locator(".sched-card", { hasText: "Weekly CRM digest" })).toHaveCount(1);

  await page.locator(".sched-card", { hasText: "Weekly CRM digest" }).click();
  await page.getByRole("button", { name: /Delete/ }).click();
  await page.getByTestId("task-delete-confirm").click();
  await expect(page.getByText(/No scheduled tasks yet/)).toBeVisible();
});
