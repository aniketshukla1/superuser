import { test, expect } from "./fixtures";

// SKILLS-SPEC §9 journey 3 — import with the mandatory review gate: the preview installs
// NOTHING; confirm installs exactly what was on screen. File import was hidden with F-9 (the
// server never had an upload route); a link is the import the server has, in the same
// two-step shape — fetch and show the instructions, then install them. Hermetic:
// preview/install round-trip through fixtures.ts state.

test("skills-upload: a link previews and installs nothing → Install skill → a real row", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Skills", exact: true }).click();

  // Add skill ▾ → Install from a link → the source field, focused.
  await page.getByRole("button", { name: /Add skill/ }).click();
  await page.getByRole("menuitem", { name: /Install from a link/ }).click();
  const source = page.getByPlaceholder(/owner\/repo/);
  await expect(source).toBeFocused();

  // A URL that is not a SKILL.md is refused in words, and nothing is staged.
  await source.fill("https://example.com/skills/greet");
  await page.getByRole("button", { name: "Fetch" }).click();
  await expect(page.getByRole("alert")).toContainText("doesn't point at a SKILL.md");

  await source.fill("acme/greet");
  await page.getByRole("button", { name: "Fetch" }).click();

  // The mandatory review screen: everything parsed, nothing installed yet.
  await expect(page.getByText("Install “greet”?")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.getByText("says hello")).toBeVisible();
  await expect(page.getByText("Say hello warmly.")).toBeVisible();
  await expect(page.getByText(/^From https:\/\/raw\.githubusercontent\.com\/acme\/greet\//)).toBeVisible();
  await expect(page.getByText("greet", { exact: true })).toHaveCount(0); // preview only, no row
  await expect(page.getByTestId("skills-tab-all")).toContainText("2");

  await page.getByRole("button", { name: "Install skill" }).click();

  // Installed: teal name-first banner and a real row; the review card is gone.
  const status = page.getByRole("status");
  await expect(status).toContainText("greet");
  await expect(status).toContainText("can now use it in every conversation");
  await expect(page.getByText("greet", { exact: true })).toHaveCount(2); // banner + the new row
  await expect(page.getByTestId("skills-tab-all")).toContainText("3");
  await expect(page.getByText("Install “greet”?")).toHaveCount(0);
});
