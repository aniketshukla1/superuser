import { test, expect } from "./fixtures";

// Guards the Settings-as-page refactor (§13, IA per UX-021): the ⚙ menu opens a full-page
// surface with a left sub-nav — General · Models · Skills · Voice input · Personas — and each
// section renders. Files is a card inside General. Personas is ON by default for Superuser
// (`showPersonas()`, flags.ts): assistants are the organising idea of this product, so the
// tab that manages them ships visible; this spec used to assert it hidden.
test("Settings opens as a full page and navigates sections", async ({ page }) => {
  await page.goto("/");

  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();

  // Full-page: left sub-nav + the General section (no modal backdrop).
  await expect(page.getByRole("heading", { name: "General" })).toBeVisible();
  await expect(page.locator(".modal-backdrop")).toHaveCount(0);
  for (const label of ["General", "Models", "Skills", "Voice input", "Personas"]) {
    await expect(page.getByRole("button", { name: label, exact: true })).toBeVisible();
  }
  // Files is the one genuinely folded tab — it's a card inside General now.
  await expect(page.getByRole("button", { name: "Files", exact: true })).toHaveCount(0);

  // The Files card lives inside General.
  await expect(page.getByText("Each conversation gets its own folder")).toBeVisible();

  await page.getByRole("button", { name: "Models", exact: true }).click();
  await expect(page.getByTestId("set-provider-openai")).toBeVisible();
});

// The Personas tab opens and manages personas. Default-on, so no flag is set here.
test("Settings: the Personas tab opens the persona manager", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Personas", exact: true }).click();
  await expect(page.getByText("Add personas")).toBeVisible();
});

// `showPersonas()` still reads the flag, so an explicit "0" must still hide the tab —
// that escape hatch is the only thing keeping the flag worth having.
test("Settings: the Personas tab can still be switched off by the flag", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("ocw.flag.personas", "0"));
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await expect(page.getByRole("button", { name: "Personas", exact: true })).toHaveCount(0);
});

// UX-021: Settings ▸ Models is the shared provider gallery (§39 components). Cards wear
// their own state (✓ Connected · used …); a vendor card opens the shared key form with the
// prefilled endpoint behind the disclosure; unconfigured providers preview their models.
test("Models: provider gallery states; vendor form previews models", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Models", exact: true }).click();

  // Card states from the fixtures: openai configured+used, anthropic configured, zai not.
  // The fixture stamps OpenAI's last use two hours before the run. timeAgo counts hours only
  // within the calendar day, so between midnight and 2 AM that same stamp reads "yesterday".
  await expect(page.getByTestId("set-provider-openai")).toContainText(
    /✓ Connected · used (2 hours ago|yesterday)/,
  );
  await expect(page.getByTestId("set-provider-anthropic")).toContainText("✓ Connected");
  await expect(page.getByTestId("set-provider-zai")).toContainText("Not set up");
  await expect(page.getByTestId("set-provider-ollama")).toContainText("No key needed");

  // The composer-picker card lists the curated models with provider tags.
  const picker = page.getByTestId("composer-picker");
  await expect(picker).toContainText("In the composer's picker");

  // Vendor form: blurb renders; the prefilled endpoint hides behind the disclosure.
  await page.getByTestId("set-provider-zai").click();
  await expect(page.getByText(/Uses Z AI's OpenAI-compatible API/)).toBeVisible();
  await page.getByTestId("set-endpoint-link").click();
  await expect(page.getByTestId("set-field-base_url")).toHaveValue("https://api.z.ai/api/paas/v4");

  // Unconfigured providers still preview their curated models (read-only, matrix labels).
  const preview = page.getByTestId("model-preview");
  await expect(preview).toContainText("Included models");
  await expect(preview).toContainText("GLM-5.2 · Z AI");

  // Back to the gallery via the crumb.
  await page.getByTestId("set-back").click();
  await expect(page.getByTestId("set-provider-openai")).toBeVisible();
});

// UX-021: a configured provider's form shows the in-field saved state and the Remove key…
// affordance; removing reverts the card to "Not set up".
test("Models: Remove key reverts a configured provider", async ({ page }) => {
  await page.goto("/");
  page.on("dialog", (d) => d.accept());
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Models", exact: true }).click();

  await page.getByTestId("set-provider-anthropic").click();
  await expect(page.getByTestId("set-saved-pill")).toContainText("Tested & saved");
  await page.getByTestId("set-remove-key").click();

  // Back on the gallery, the card has forgotten its key.
  await expect(page.getByTestId("set-provider-anthropic")).toContainText("Not set up");
});

// Token savings (owner ask 2026-07-17; moved under Models by UX-021): the card renders with
// the PDF fallback segmented control + attach thresholds, and edits POST through.
test("Settings: Token savings card edits PDF fallback and thresholds", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("account-row").click();
  await page.getByTestId("account-menu").getByRole("menuitem", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Models", exact: true }).click();

  const card = page.getByTestId("token-savings-card");
  await expect(card).toBeVisible();
  await expect(card.getByText("Token savings")).toBeVisible();

  // Fallback mode: fixture says "text"; switching marks "Send page images" active.
  const seg = page.getByTestId("pdf-fallback");
  await expect(seg.getByRole("button", { name: "Extract text" })).toHaveClass(/active/);
  const [req] = await Promise.all([
    page.waitForRequest((r) => r.url().endsWith("/v1/settings/pdf") && r.method() === "POST"),
    seg.getByRole("button", { name: "Send page images" }).click(),
  ]);
  expect(req.postDataJSON()).toEqual({ pdf_fallback: "images" });
  await expect(seg.getByRole("button", { name: "Send page images" })).toHaveClass(/active/);

  // Thresholds: fixture starts at 2 pages / 10 MB; editing pages POSTs the clamped value.
  await expect(card.getByTestId("pdf-max-pages")).toHaveValue("2");
  await expect(card.getByTestId("pdf-max-mb")).toHaveValue("10");
  const [req2] = await Promise.all([
    page.waitForRequest((r) => r.url().endsWith("/v1/settings/pdf") && r.method() === "POST"),
    card.getByTestId("pdf-max-pages").fill("30"),
  ]);
  expect(req2.postDataJSON()).toEqual({ pdf_max_pages: 30 });
});
