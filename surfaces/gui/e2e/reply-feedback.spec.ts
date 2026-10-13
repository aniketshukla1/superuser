import { test, expect } from "./fixtures";

// A reply's action row (copy, retry, the thumbs, the time) hangs in the gap under the reply,
// on a zero-height strip. "What was wrong?" opens in the flow under it. These are layout
// facts, so they are checked in a real browser: jsdom has no layout to measure.

const NOW = Math.floor(Date.now() / 1000);
const REASONS = [
  { id: "made_it_up", label: "It made something up" },
  { id: "wrong_source", label: "It used the wrong file or memory" },
];

async function openReply(page: import("@playwright/test").Page, given: unknown[] = []) {
  await page.route("**/v1/sessions/*/messages", (route) =>
    route.fulfill({
      json: {
        messages: [
          { role: "user", content: "Summarise the launch note", at: NOW - 120 },
          { role: "assistant", content: "Here is the summary.\n\n- The date moved to Thursday\n- Pricing is unchanged", at: NOW - 100 },
        ],
      },
    }),
  );
  await page.route("**/v1/feedback*", (route) => route.fulfill({ json: { reasons: REASONS, given } }));
  await page.goto("/");
  const reply = page.locator(".bubble-assistant").last();
  await expect(reply).toContainText("Pricing is unchanged");
  return reply;
}

test("the thumbs sit in the reply's action row, in line with copy", async ({ page }) => {
  const reply = await openReply(page);
  await reply.hover();
  const copy = (await reply.getByTestId("bubble-copy").boundingBox())!;
  const up = (await reply.getByLabel("This was good").boundingBox())!;
  const down = (await reply.getByLabel("Something was wrong").boundingBox())!;
  // One row: the same line, left to right, nothing on top of anything else.
  expect(Math.abs(up.y + up.height / 2 - (copy.y + copy.height / 2))).toBeLessThanOrEqual(2);
  expect(up.x).toBeGreaterThanOrEqual(copy.x + copy.width);
  expect(down.x).toBeGreaterThanOrEqual(up.x + up.width);
});

test("what-was-wrong opens below the action row, not under it", async ({ page }) => {
  const reply = await openReply(page);
  await reply.hover();
  await reply.getByLabel("Something was wrong").click();
  const panel = reply.locator(".feedback-reasons");
  await expect(panel).toBeVisible();
  await reply.hover();
  const copy = (await reply.getByTestId("bubble-copy").boundingBox())!;
  const box = (await panel.boundingBox())!;
  // The row is still there on hover (copy, the time); the panel starts clear of it.
  expect(box.y).toBeGreaterThanOrEqual(copy.y + copy.height + 2);
});

test("a verdict already given hides with the row until the reply is hovered", async ({ page }) => {
  // Owner ask (2026-09-24): a thumb you pressed does not stay showing under the reply.
  const reply = await openReply(page, [{ message: 1, verdict: "up", message_at: NOW - 100 }]);
  const up = reply.getByLabel("This was good");
  await expect(up).toHaveAttribute("aria-pressed", "true");
  const shown = () => up.evaluate((el) => {
    let o = 1;
    for (let e: Element | null = el; e; e = e.parentElement) o *= parseFloat(getComputedStyle(e).opacity);
    return o;
  });
  await page.mouse.move(2, 2);
  await expect.poll(shown).toBe(0);
  await reply.hover();
  await expect.poll(shown).toBe(1);
});
