import { test, expect } from "./fixtures";

// SKILLS-SPEC §9 journey 1 — Settings ▸ Skills as the management home: the Add-skill menu
// offers the doors that work, Create with Superuser hands off to a conversation, and a row
// disables with the amber clean-slate banner or deletes in two steps. Hermetic: every /v1
// call lands in fixtures.ts.
//
// The write form, file import, the editor and the "Show folder" chip were built against
// routes the server never had; they were hidden on the owner's call (2026-09-22) until
// they are built.

const openSkills = async (page: import("@playwright/test").Page) => {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Skills", exact: true }).click();
};

test("skills-settings: the menu offers the two doors that work; Create hands off to a chat", async ({ page }) => {
  await openSkills(page);

  // The seeded rows render with their provenance; the list is the page (no standing
  // add-surfaces), and nothing on it calls a route the server lacks.
  await expect(page.getByText("weekly-report")).toBeVisible();
  await expect(page.getByText("uploaded")).toBeVisible();
  await expect(page.getByText("Start a conversation")).toHaveCount(0);
  await expect(page.getByTitle("Show folder")).toHaveCount(0);
  await expect(page.getByTitle("Edit")).toHaveCount(0);

  // Add skill ▾ → exactly two doors: Install from a link, and Create with Superuser.
  await page.getByRole("button", { name: /Add skill/ }).click();
  const doors = page.getByRole("menu").getByRole("menuitem");
  await expect(doors).toHaveCount(2);
  await expect(doors.nth(0)).toContainText("Install from a link");
  await expect(doors.nth(1)).toContainText("Create with Superuser");
  await expect(page.getByText("Write it myself")).toHaveCount(0);
  await expect(page.getByText("Import a file")).toHaveCount(0);

  // Creation is a conversation (SKILLS-SPEC §5.2): a fresh chat, the composer holding the
  // sentence to finish — the assistant builds the skill and asks before adding it.
  await doors.nth(1).click();
  await expect(page.getByTestId("composer-input")).toHaveValue(/^Build a new skill for me: /);
  await expect(page.getByRole("heading", { name: "Skills" })).toHaveCount(0);
});

test("skills-settings: disable → amber everywhere/clean-slate banner; delete is two-step", async ({ page }) => {
  await openSkills(page);

  await page.getByLabel("weekly-report enabled").click();
  const status = page.getByRole("status");
  await expect(status).toContainText("weekly-report");
  await expect(status).toContainText("turned off everywhere");
  await expect(status).toContainText("start a new one for a completely clean slate");

  // Two-step delete: arm, confirm, row gone, banner names the skill.
  await page.getByLabel("Delete html-to-markdown").click();
  await expect(page.getByText("html-to-markdown")).toBeVisible(); // armed ≠ deleted
  await page.getByText("Confirm delete").click();
  await expect(page.getByText("html-to-markdown")).toHaveCount(1); // only the banner remains
  await expect(page.getByRole("status")).toContainText("removed");
});
