// UX-023: automations get sidebar presence — an "Automations" nav row and a "Scheduled"
// band with ONE entry per automation (name + cadence + unseen-runs badge). Opening an
// automation's detail marks it seen: the badge clears immediately via the
// AUTOMATIONS_CHANGED broadcast, and runs newer than the pre-open mark are counted as new
// inside the detail. Since the rail became two rows and a More (2026-08-24), the nav row
// lives in the More menu on a fresh install.
import { expect, type Page } from "@playwright/test";
import { test } from "./fixtures";

/** The Automations nav row, from the rail's More menu. */
async function automationsNav(page: Page) {
  await page.getByTestId("nav-more").click();
  return page.getByTestId("more-menu").getByTestId("nav-automations");
}

test("nav row + Scheduled band render with unseen badges; runs stay out of Recent", async ({
  page,
}) => {
  await page.goto("/");

  // The nav row carries no badge of its own (owner call: the Scheduled entry alone
  // carries the count) — and neither does More on its behalf.
  await expect(page.getByTestId("nav-more")).not.toContainText("2");
  const nav = await automationsNav(page);
  await expect(nav).toBeVisible();
  await expect(nav).toContainText("Automations");
  await expect(nav).not.toContainText("2");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("more-menu")).toHaveCount(0);

  // Scheduled band: one entry PER AUTOMATION — never per run. The noisy task wears
  // its badge; the quiet one shows none.
  const band = page.getByTestId("scheduled-band");
  await expect(band.getByTestId("scheduled-task-1")).toContainText("Daily AI News");
  await expect(band.getByTestId("scheduled-task-1")).toContainText("2");
  await expect(band.getByTestId("scheduled-task-2")).toContainText("Weekly CRM digest");
  await expect(band.getByTestId("scheduled-task-2")).not.toContainText("2");

  // Runs never appear as session rows (their sessions are __run__-prefixed and the
  // server hides them) — the band's entries are the only automation presence.
  await expect(page.getByTitle("__run__r1")).toHaveCount(0);
});

test("opening a Scheduled entry lands on the detail, marks seen, clears the badge", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByTestId("scheduled-task-1").click();

  // The Automations surface opens ON that automation's detail…
  await expect(page.getByRole("heading", { name: "Daily AI News" })).toBeVisible();
  // …runs newer than the pre-open seen mark wear the "new" pill…
  await expect(page.getByTestId("run-new").first()).toBeVisible();
  // …and the entry's badge clears without waiting for any poll (mark-seen broadcast).
  await expect(page.getByTestId("scheduled-task-1")).not.toContainText("2");
});

test("the nav row opens the Automations overview", async ({ page }) => {
  await page.goto("/");
  await (await automationsNav(page)).click();
  await expect(page.getByRole("heading", { name: "Scheduled" })).toBeVisible();
  // The menu closes behind the choice, and More lights up for the surface it holds.
  await expect(page.getByTestId("more-menu")).toHaveCount(0);
  await expect(page.getByTestId("nav-more")).toHaveClass(/bg-line/);
});

test("deleting an automation clears the band at once; nav re-entry lands on the list", async ({
  page,
}) => {
  await page.goto("/");
  // Open the automation from the band, delete it from the detail.
  await page.getByTestId("scheduled-task-2").click();
  await expect(page.getByRole("heading", { name: "Weekly CRM digest" })).toBeVisible();
  // Delete asks first on the detail page, as it does on the list card.
  await page.getByRole("button", { name: /Delete/ }).click();
  await expect(page.getByTestId("scheduled-task-2")).toHaveCount(1); // asked ≠ deleted
  await page.getByTestId("task-delete-confirm").click();

  // The Scheduled band drops the entry immediately (broadcast, not the 15s poll)…
  await expect(page.getByTestId("scheduled-task-2")).toHaveCount(0);

  // …and after visiting a session, the nav row must land on the OVERVIEW — the
  // remembered detail target for a deleted automation once left "Loading…" forever.
  await page.getByTitle("Weekly plan 1").click();
  await (await automationsNav(page)).click();
  await expect(page.getByRole("heading", { name: "Scheduled" })).toBeVisible();
  await expect(page.getByText("Loading…")).toHaveCount(0);
});
