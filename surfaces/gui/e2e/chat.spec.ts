import { test, expect } from "./fixtures";

// The core loop: boot-resume into the last session, send a message over the WebSocket, and render
// the streamed reply — plus the in-session approval round-trip (permission_required suspends the
// turn until Allow/Deny goes back over the socket). The fake agent lives in fixtures.ts.

test("send → user bubble → streamed echo reply renders", async ({ page }) => {
  await page.goto("/");

  // Boot resumes the most recent session ("Draft the launch note") and connects; the composer is
  // live once the fake agent's `ready` lands.
  const box = page.getByTestId("composer-input");
  await expect(box).toBeVisible();

  await box.fill("hello agent");
  await page.getByRole("button", { name: "Send" }).click();

  // Local echo of the user message, then the agent's reply (delta-streamed, then finalized).
  await expect(page.getByText("hello agent", { exact: true }).first()).toBeVisible();
  await expect(page.getByText(/Echo: hello agent/)).toBeVisible();
  // The message carried the composer's visible model (model-per-message contract): what the
  // user sees at send time is exactly what serves the turn.
  await expect(page.getByText("[model=anthropic:claude-opus-4-8]")).toBeVisible();
  // …and the picker STAYS actionable after the first turn (§17 rev 2026-07-22 — mid-session
  // switching shipped). The chip is the one place the model reads: the header names who and
  // where, not which model (owner ask 2026-08-24).
  await expect(page.getByTestId("model-chip")).toBeEnabled();
  await expect(page.getByTestId("model-chip")).toContainText("Claude Opus 4.8");
  // Composer cleared and re-armed for the next turn.
  await expect(box).toHaveValue("");
});

test("approval: tool request suspends the turn; Allow once resumes it", async ({ page }) => {
  await page.goto("/");
  const box = page.getByTestId("composer-input");
  await expect(box).toBeVisible();

  await box.fill("please run a tool");
  await page.getByRole("button", { name: "Send" }).click();

  // The approval card surfaces the tool + reason and blocks until a decision.
  await expect(page.getByText("The coworker wants to run a command.").first()).toBeVisible();
  await page.getByRole("button", { name: "Allow once" }).last().click();

  // Decision goes back over the socket; the agent finishes the tool and the turn.
  await expect(page.getByText("The command ran; 1 file found.")).toBeVisible();
});

test("approval: Deny skips the tool and the agent says so", async ({ page }) => {
  await page.goto("/");
  const box = page.getByTestId("composer-input");
  await expect(box).toBeVisible();

  await box.fill("please run a tool");
  await page.getByRole("button", { name: "Send" }).click();

  await expect(page.getByRole("button", { name: "Deny" }).last()).toBeVisible();
  await page.getByRole("button", { name: "Deny" }).last().click();
  await expect(page.getByText("Understood — skipped the command.")).toBeVisible();
});

test("long user pastes clamp with a Show more/Show less toggle", async ({ page }) => {
  await page.goto("/");
  const box = page.getByTestId("composer-input");
  await expect(box).toBeVisible();

  const tail = "END-OF-PASTE-MARKER";
  const paste =
    "reply OK. " + "lorem ipsum dolor sit amet consectetur ".repeat(60) + tail; // ~2.4k chars
  await box.fill(paste);
  await page.getByRole("button", { name: "Send" }).click();

  // Folded at eight LINES by CSS (line-clamp), not cut at a character count: the whole paste
  // stays in the bubble — it is what copy takes and what the model received — and the box
  // is shorter than what it holds.
  const toggle = page.getByTestId("user-show-more");
  await expect(toggle).toHaveText("Show more");
  const clamp = page.locator(".bubble-user").last().locator(".user-clamp");
  await expect(clamp).toContainText("reply OK.");
  await expect(clamp).toContainText(tail);
  await expect(clamp).toHaveClass(/is-clamped/);
  const overflows = () => clamp.evaluate((el) => el.scrollHeight > el.clientHeight + 2);
  expect(await overflows()).toBe(true);

  // Expand → the whole paste shows and the control says "Show less"; collapse → folded again.
  await toggle.click();
  await expect(clamp).toHaveClass(/is-open/);
  await expect(toggle).toHaveText("Show less");
  expect(await overflows()).toBe(false);
  await toggle.click();
  await expect(clamp).toHaveClass(/is-clamped/);
  await expect(toggle).toHaveText("Show more");

  // Short messages never show the control.
  await expect(page.getByText("Echo:").first()).toBeVisible();
  await box.fill("short follow-up");
  await page.getByRole("button", { name: "Send" }).click();
  // The bubble, not the text: the transcript's map of questions carries the same words.
  await expect(page.locator(".bubble-user").last()).toContainText("short follow-up");
  await expect(page.getByTestId("user-show-more")).toHaveCount(1); // still only the paste's
});
