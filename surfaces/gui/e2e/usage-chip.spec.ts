// Token usage in the composer (OPE-42). The usage chip became the context ring in the
// 2026-08-26 composer rebuild: an 18px gauge of how full the model's context window is,
// always on the row, opening a panel with the window, the tokens this app has spent (the
// server's rollup), and the per-session, per-model breakdown the chip's popover used to hold.
// The fake agent attaches fixed usage to every echo turn (input 1k / output 200 /
// cache_read 8k / cache_write 800 — 10k per turn), and the settings fixture maps the default
// model to a 200k context window.
//
// The Settings toggle that switched the chip between a bar and the session total went with
// the chip: the ring never read it, so it saved a choice nothing honoured.
import { expect } from "@playwright/test";
import { test } from "./fixtures";

test("the context ring fills after a turn and its panel carries the per-model breakdown", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();

  // Fresh session: the ring is on the row (it is also the way to this app's totals) but empty.
  const ring = page.getByTestId("context-ring");
  await expect(ring).toHaveAttribute("title", "Context window 0% full");
  await expect(ring.locator(".composer-ring-fill")).toHaveCount(0);

  const box = page.getByTestId("composer-input");
  await box.fill("hello");
  await box.press("Enter");
  await expect(page.getByText("Echo: hello", { exact: false }).first()).toBeVisible({
    timeout: 10_000,
  });

  // The fill is the prompt side of the last request (1k + 8k + 800 = 9.8k) of 200k = 5%.
  await expect(ring).toHaveAttribute("title", "Context window 5% full");
  await expect(ring.locator(".composer-ring-fill")).toHaveCount(1);

  // Panel: the window, then the tokens through this app — with the turn that reported
  // nothing said rather than folded in — then the breakdown, on request.
  await ring.click();
  const panel = page.getByTestId("context-menu");
  await expect(panel).toContainText("9.8k / 200k (5%)");
  await expect(panel).toContainText("Tokens through this app");
  await expect(panel.getByTestId("usage-today")).toContainText("34.5k");
  await expect(panel.getByTestId("usage-today")).toContainText("1 of 4 turns didn’t report a cost");
  await expect(panel.getByTestId("usage-this week")).toContainText("456k");

  await expect(panel.getByTestId("usage-by-model")).toHaveCount(0);
  await panel.getByTestId("usage-breakdown").click();
  const table = panel.getByTestId("usage-by-model");
  await expect(table).toContainText("Claude Opus 4.8 · Anthropic");
  // Cache split present → the input rows read as components of Total input.
  await expect(table).toContainText("Uncached input");
  await expect(table).toContainText("Cache reads");
  await expect(table).toContainText("Cache writes");
  // Total input = fresh 1k + cache_read 8k + cache_write 800 (cumulative billed input).
  await expect(table).toContainText("Total input");
  await expect(table).toContainText("9.8k");
  await expect(table).toContainText("10k tokens");

  // Escape closes the panel; a second turn doubles the session totals, while the window —
  // per request, not a running sum — stays at 5%.
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await box.fill("again");
  await box.press("Enter");
  await expect(page.getByText("Echo: again", { exact: false }).first()).toBeVisible({
    timeout: 10_000,
  });
  await ring.click();
  await panel.getByTestId("usage-breakdown").click();
  await expect(panel.getByTestId("usage-by-model")).toContainText("20k tokens");
  await expect(panel).toContainText("9.8k / 200k (5%)");
});

test("usage resets on a new session", async ({ page }) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();
  const box = page.getByTestId("composer-input");
  await box.fill("hello");
  await box.press("Enter");
  const ring = page.getByTestId("context-ring");
  await expect(ring).toHaveAttribute("title", "Context window 5% full", { timeout: 10_000 });

  // "New session" wipes the transcript — and the usage accumulation with it.
  await page.getByRole("button", { name: "New session", exact: true }).click();
  await expect(ring).toHaveAttribute("title", "Context window 0% full");
  await ring.click();
  await expect(page.getByTestId("context-menu")).toContainText("Tokens through this app");
  await expect(page.getByTestId("usage-breakdown")).toHaveCount(0);
});
