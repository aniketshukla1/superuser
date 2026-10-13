import { test, expect } from "./fixtures";

// SKILLS-SPEC §9 journey 2 — liveness from the session's seat: the composer's "/" popup is
// the live "what can my assistant use right now" view. A skill installed in Settings is
// offered; a disabled one vanishes. Hermetic: the popup reads /v1/sessions/{id}/skills from
// fixtures. Rows carry the bare name, with what the skill does beside it.

test("skills-session: newly installed skill offered in '/', disabled one absent", async ({ page }) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();

  // The seeded menu: both enabled skills offered on "/".
  const box = page.getByTestId("composer-input");
  const popup = page.getByTestId("skill-popup");
  const offered = (name: string) => popup.getByRole("option", { name: new RegExp(`^${name} `) });
  await box.fill("/");
  await expect(popup).toBeVisible();
  await expect(offered("weekly-report")).toBeVisible();
  await expect(offered("html-to-markdown")).toBeVisible();
  await box.fill(""); // close the popup

  // Settings round-trip: install one skill from a link (the add door the page has — the
  // write-it-yourself form went with F-9), disable another.
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Skills", exact: true }).click();
  await page.getByRole("button", { name: /Add skill/ }).click();
  await page.getByRole("menuitem", { name: /Install from a link/ }).click();
  await page.getByPlaceholder(/owner\/repo/).fill("acme/fresh-skill");
  await page.getByRole("button", { name: "Fetch" }).click();
  await page.getByRole("button", { name: "Install skill" }).click();
  await expect(page.getByRole("status")).toContainText("fresh-skill");
  await page.getByLabel("weekly-report enabled").click();
  await expect(page.getByRole("status")).toContainText("turned off everywhere");

  // Back in the session: the popup reflects the new state — installed offered, disabled gone.
  await page.getByText("Draft the launch note").first().click();
  await box.fill("/");
  await expect(popup).toBeVisible();
  await expect(offered("fresh-skill")).toBeVisible();
  await expect(offered("weekly-report")).toHaveCount(0);
  await expect(offered("html-to-markdown")).toBeVisible(); // untouched one persists
});
