import { test, expect } from "./fixtures";

// Guards the composer row (§22, rebuilt 2026-08-26): send-gating (accent only with content),
// the "+" add menu, and the Mode menu (ways of working, the guarded bypass below a divider,
// and the folded-in Send-to-Inbox toggle).
test("composer: send-gating, + add menu, Mode menu", async ({ page }) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();

  const box = page.getByTestId("composer-input");
  const send = page.getByRole("button", { name: "Send" });

  // Send is subtle grey when empty, accent once there's content, grey again when cleared.
  await expect(send).not.toHaveClass(/bg-accent/);
  await box.fill("hello there");
  await expect(send).toHaveClass(/bg-accent/);
  await box.fill("");
  await expect(send).not.toHaveClass(/bg-accent/);

  // "+" add menu: one "Add files or photos" (the picker asks the type, so the menu no longer
  // does), Skills, and Connectors. Add folder needs a native picker — desktop app only.
  await page.getByRole("button", { name: "Add to this message" }).click();
  const add = page.getByTestId("attach-menu");
  await expect(add.getByRole("menuitem", { name: /Add files or photos/ })).toBeVisible();
  await expect(add.getByRole("menuitem", { name: /Skills/ })).toBeVisible();
  await expect(add.getByRole("menuitem", { name: /Connectors/ })).toBeVisible();
  await expect(add.getByTestId("attach-folder")).toHaveCount(0);
  // Clicking the backdrop closes it.
  await page.getByTestId("menu-backdrop").click();
  await expect(page.getByTestId("attach-menu")).toHaveCount(0);

  // Mode menu: the ways of working with the current one checked — Plan is back (owner ask
  // 2026-08-26), Custom stays hidden — then, below a divider, the bypass with its own verb,
  // plus the Unattended/send-to-Inbox toggle (§22).
  await page.getByRole("button", { name: "Mode", exact: true }).click();
  const menu = page.getByTestId("mode-menu");
  await expect(menu.getByTestId("mode-discuss")).toBeVisible();
  await expect(menu.getByTestId("mode-plan")).toBeVisible();
  await expect(menu.getByText("Custom", { exact: true })).toHaveCount(0);
  await expect(menu.getByTestId("mode-interactive")).toHaveAttribute("aria-checked", "true");
  await expect(menu.getByTestId("mode-discuss")).toHaveAttribute("aria-checked", "false");
  await expect(menu.getByRole("switch", { name: "Send approvals to the Inbox" })).toBeVisible();
  // Switching the asking off asks first, in place, and backing out changes nothing.
  await menu.getByTestId("mode-auto").click();
  await expect(menu.getByTestId("mode-bypass-confirm")).toContainText("Run everything without asking?");
  await menu.getByRole("button", { name: "Keep asking" }).click();
  await expect(menu.getByTestId("mode-bypass-confirm")).toHaveCount(0);
  await expect(menu.getByTestId("mode-interactive")).toHaveAttribute("aria-checked", "true");
  // Picking a way of working closes the menu (and would flip the live engine's mode).
  await menu.getByTestId("mode-discuss").click();
  await expect(page.getByTestId("mode-menu")).toHaveCount(0);
});

// PDFs read as data URLs and show a named chip (DMG #29 walkthrough catch: PDFs silently
// no-op'd because readFile only handled images and text).
test("composer: picking a PDF shows an attachment chip and arms send", async ({ page }) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();

  const send = page.getByRole("button", { name: "Send" });
  await expect(send).not.toHaveClass(/bg-accent/);

  await page.locator('input[type="file"]').setInputFiles({
    name: "report.pdf",
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF"),
  });

  const chip = page.locator(".attach-chip");
  await expect(chip).toContainText("report.pdf");
  await expect(send).toHaveClass(/bg-accent/); // attachment alone arms send

  // Removing the chip disarms send again.
  await chip.locator(".attach-x").click();
  await expect(page.locator(".attach-chip")).toHaveCount(0);
  await expect(send).not.toHaveClass(/bg-accent/);
});

// Token-savings threshold (owner ask, 2026-07-17): a PDF over the user's page limit is
// REJECTED with a visible notice — no chip, send stays disarmed. Fixture limit: 2 pages;
// the mock inspect endpoint reads the page count from a "%%pages=N" marker in the body.
test("composer: PDF over the page threshold is rejected with a notice", async ({ page }) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();

  await page.locator('input[type="file"]').setInputFiles({
    name: "big-report.pdf",
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.4\n%%pages=34\ntrailer\n<<>>\n%%EOF"),
  });

  const notice = page.getByTestId("attach-notice");
  await expect(notice).toContainText("big-report.pdf skipped");
  await expect(notice).toContainText("34 pages is over your 2-page limit");
  await expect(page.locator(".attach-chip")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Send" })).not.toHaveClass(/bg-accent/);

  // The ✕ dismisses the notice.
  await notice.getByRole("button").click();
  await expect(page.getByTestId("attach-notice")).toHaveCount(0);

  // A small PDF (1 page per the mock) still attaches fine after a rejection.
  await page.locator('input[type="file"]').setInputFiles({
    name: "small.pdf",
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.4\n%%pages=1\ntrailer\n<<>>\n%%EOF"),
  });
  await expect(page.locator(".attach-chip")).toContainText("small.pdf");
});
