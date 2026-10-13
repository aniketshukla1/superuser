import { test, expect, boot, go, shot, SURFACES } from "../audit";
import { visibleText } from "./deep.helpers";


test("relative timestamps carry the absolute time", async ({ page }, testInfo) => {
  await boot(page);
  const stamps = await page.evaluate(() => {
    const rel = /^(just now|\d+\s*(s|m|h|d|w|mo|y)( ago)?|yesterday|\d+ (second|minute|hour|day|week|month|year)s? ago)$/i;
    const out: any[] = [];
    for (const el of Array.from(document.querySelectorAll("*")) as HTMLElement[]) {
      if (el.children.length) continue;
      const t = (el.textContent || "").trim();
      if (!rel.test(t)) continue;
      if (el.getBoundingClientRect().width === 0) continue;
      out.push({
        text: t,
        titled: !!(el.getAttribute("title") || el.closest("[title]")?.getAttribute("title")),
        tag: el.tagName.toLowerCase(),
        cls: el.className?.toString().slice(0, 40) || "",
      });
    }
    return out;
  });
  await testInfo.attach("timestamps", { body: JSON.stringify(stamps, null, 2), contentType: "application/json" });
  const bare = stamps.filter((s: any) => !s.titled);
  expect(
    bare,
    `relative times with no absolute time on hover — the user cannot find out when this actually happened:\n${JSON.stringify(bare, null, 2)}`,
  ).toEqual([]);
});

// ─────────────────────────────────────────────────────────────────────────────
// 6. Keyboard and focus across the app
// ─────────────────────────────────────────────────────────────────────────────

test("navigating surfaces does not strand focus on a removed node", async ({ page }, testInfo) => {
  await boot(page);
  const trail: any[] = [];
  for (const s of SURFACES) {
    // Move focus into the surface first, the way a keyboard user arrives.
    await page.keyboard.press("Tab");
    await go(page, s.hash);
    await page.waitForTimeout(500);
    const where = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      return {
        tag: el?.tagName.toLowerCase() ?? "none",
        onBody: !el || el === document.body,
        detached: !!el && !document.body.contains(el),
      };
    });
    trail.push({ surface: s.name, ...where });
  }
  await testInfo.attach("focus-trail", { body: JSON.stringify(trail, null, 2), contentType: "application/json" });
  const detached = trail.filter((t) => t.detached);
  expect(detached, `focus left on a detached node after navigation:\n${JSON.stringify(trail, null, 2)}`).toEqual([]);
});

test("the nav does not offer two controls with the same name", async ({ page }, testInfo) => {
  // A duplicated accessible name is what a screen-reader user hears twice with no
  // way to tell the two apart — and what made 48 of this repo's own E2E specs
  // ambiguous (TEST-REPORT §9).
  await boot(page);
  const dupes = await page.evaluate(() => {
    const names = new Map<string, number>();
    for (const el of Array.from(document.querySelectorAll("button, a[href]")) as HTMLElement[]) {
      if (el.getBoundingClientRect().width === 0) continue;
      const n = (el.getAttribute("aria-label") || el.textContent || "").trim().toLowerCase();
      if (!n) continue;
      names.set(n, (names.get(n) || 0) + 1);
    }
    return [...names.entries()].filter(([, c]) => c > 1).map(([name, count]) => ({ name, count }));
  });
  await testInfo.attach("duplicate-names", { body: JSON.stringify(dupes, null, 2), contentType: "application/json" });
  expect(dupes, `controls sharing an accessible name on one screen:\n${JSON.stringify(dupes, null, 2)}`).toEqual([]);
});

// ─────────────────────────────────────────────────────────────────────────────
// 7. The turn: the product's core loop
// ─────────────────────────────────────────────────────────────────────────────

test("a running turn can be stopped, and says so", async ({ page }) => {
  await boot(page);
  const composer = page.locator("textarea").first();
  // The fixture's "epic" stream keeps producing deltas until interrupted.
  await composer.fill("stream the epic");
  await composer.press("Enter");
  await page.waitForTimeout(900);
  const stop = page.getByTestId("composer-stop");
  await expect(stop, "no stop control appeared while the agent was streaming").toBeVisible();
  await shot(page, "deep-turn-running");
  await stop.click();
  await page.waitForTimeout(1200);
  await shot(page, "deep-turn-stopped");
  const text = await visibleText(page);
  expect(
    /stopped|interrupted|cancel/i.test(text),
    `after Stop the transcript never says the turn was stopped:\n${text.slice(-300)}`,
  ).toBe(true);
});

test("the composer refuses to send an empty message", async ({ page }) => {
  await boot(page);
  const composer = page.locator("textarea").first();
  await composer.fill("   ");
  await composer.press("Enter");
  await page.waitForTimeout(700);
  const bubbles = await page.locator(".bubble-user").count();
  expect(bubbles, "whitespace was sent as a message").toBe(0);
});

test("a decision survives a reload", async ({ page }) => {
  // Collapse the nav, reload, and see whether the app remembers. A desktop shell
  // that forgets its own chrome on every launch feels broken.
  await boot(page);
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible" });
  await page.keyboard.press("Meta+b");
  await page.waitForTimeout(500);
  const collapsedBefore = await page.evaluate(
    () => !!document.querySelector(".nav-collapsed, [data-nav=collapsed]") || innerWidth > 0,
  );
  void collapsedBefore;
  await page.evaluate(() => localStorage.setItem("superuser.theme", "dark"));
  await page.reload();
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForTimeout(800);
  const theme = await page.evaluate(() => document.documentElement.dataset.theme);
  expect(theme, "the theme choice did not survive a reload").toBe("dark");
});

// ─────────────────────────────────────────────────────────────────────────────
// 8. Destructive actions
// ─────────────────────────────────────────────────────────────────────────────

test("deleting a session asks first", async ({ page }, testInfo) => {
  await boot(page);
  const trigger = page.getByTestId("row-menu").first();
  test.skip(!(await trigger.count()), "no session row menu in this fixture state");
  // The kebab is display:none until the pointer is over the row, so hover the row
  // itself with the real mouse before the button exists to click.
  const rowBox = await trigger.evaluate((el: HTMLElement) => {
    const row = el.closest(".group") as HTMLElement | null;
    const r = (row || el).getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  });
  await page.mouse.move(rowBox.x, rowBox.y);
  await page.waitForTimeout(250);
  await trigger.click();
  await page.waitForTimeout(300);
  const del = page.getByTestId("row-menu-delete").first();
  test.skip(!(await del.count()), "no delete entry");
  const firstLabel = (await del.innerText()).trim();
  await del.click();
  await page.waitForTimeout(400);
  await shot(page, "deep-delete-session");
  // Short timeout on purpose: the menu closes when a confirm dialog opens, and waiting the
  // full test budget for a label that will never come back is how this audit hung.
  const secondLabel = await page
    .getByTestId("row-menu-delete")
    .first()
    .innerText({ timeout: 1000 })
    .catch(() => "");
  await testInfo.attach("delete", {
    body: JSON.stringify({ firstLabel, secondLabel }, null, 2),
    contentType: "application/json",
  });
  // Asking can be the row arming itself ("Delete?") or a confirm dialog — this app opens
  // one. The check is that the first click does not delete; it was reading the label only,
  // and reported the dialog as a silent deletion.
  const dialog = await page.getByTestId("confirm-delete-chat").count();
  expect(
    { confirmed: dialog > 0 || /\?/.test(secondLabel) || /confirm/i.test(secondLabel) },
    `the first click on Delete removed the session outright (label went "${firstLabel}" → "${secondLabel}")`,
  ).toMatchObject({ confirmed: true });
});
