import { test, expect, boot, go, shot, watchErrors } from "../audit";
import { visibleText } from "./deep.helpers";


test("session row actions are reachable without a mouse", async ({ page }, testInfo) => {
  // Rename / Pin / Archive / Delete live behind a kebab that is `hidden
  // group-hover:flex` — display:none until the pointer is over the row. Nothing
  // display:none is focusable, so Tab can never reach it.
  await boot(page);
  const facts = await page.evaluate(() => {
    const kebabs = Array.from(document.querySelectorAll("[data-testid=row-menu]")) as HTMLElement[];
    return {
      count: kebabs.length,
      displayed: kebabs.filter((k) => getComputedStyle(k).display !== "none").length,
      // A `focus-within` escape hatch would make it keyboard-reachable.
      wrapperClasses: kebabs.slice(0, 2).map((k) => k.parentElement?.className?.toString().slice(0, 80) ?? ""),
      focusableViaTab: kebabs.filter((k) => {
        const r = k.getBoundingClientRect();
        return r.width > 0 && r.height > 0;
      }).length,
    };
  });
  await testInfo.attach("row-actions", { body: JSON.stringify(facts, null, 2), contentType: "application/json" });
  expect(
    facts.focusableViaTab,
    `every session's rename/pin/archive/delete menu is mouse-only:\n${JSON.stringify(facts, null, 2)}`,
  ).toBeGreaterThan(0);
});

// ─────────────────────────────────────────────────────────────────────────────
// 9. Live regions for the things that block the user
// ─────────────────────────────────────────────────────────────────────────────

test("turning an automation off against a failing backend does not lie", async ({ page }, testInfo) => {
  // A second, independent write path — a scheduled task's on/off switch. Higher
  // stakes than a preference: the user believes they have disarmed something.
  const errors = watchErrors(page);
  await boot(page);
  await go(page, "#/automations");
  await page.waitForTimeout(700);
  await page.route("**/v1/**", async (r) => {
    if (r.request().method() === "GET") return r.fallback();
    return r.fulfill({ status: 500, contentType: "application/json", body: '{"error":"boom"}' });
  });
  // The on/off switch lives on the automation's detail page.
  const card = page.locator(".sched-card, [class*=sched-card]").first();
  test.skip(!(await card.count()), "no automation in this fixture state");
  await card.click();
  await page.waitForTimeout(700);
  // The native input is visually hidden behind a `.slider` span; the label is the
  // real hit target.
  const sw = page.locator('input[type="checkbox"]').first();
  const label = page.locator("label.switch").first();
  test.skip(!(await sw.count()), "no automation switch on the detail page");
  const before = String(await sw.isChecked());
  await label.click();
  await page.waitForTimeout(1500);
  const after = String(await sw.isChecked());
  await testInfo.attach("automation-toggle", {
    body: JSON.stringify({ before, after }, null, 2),
    contentType: "application/json",
  });
  const live = await page.evaluate(() =>
    (Array.from(document.querySelectorAll("[aria-live], [role=alert], [role=status]")) as HTMLElement[])
      .filter((r) => r.getBoundingClientRect().height > 0)
      .map((r) => ({ role: r.getAttribute("role"), live: r.getAttribute("aria-live"), text: (r.textContent || "").trim().slice(0, 60) })),
  );
  await testInfo.attach("toast", {
    body: JSON.stringify({ live, errors }, null, 2),
    contentType: "application/json",
  });
  await shot(page, "deep-error-toast");
  const text = await visibleText(page);
  expect(
    { told: /couldn'?t|could not|failed|error|try again/i.test(text), before, after },
    `the switch was clicked, the server refused, and nothing was said (switch ${before} → ${after}). The click simply appears not to have worked. Live regions present: ${JSON.stringify(live)}`,
  ).toMatchObject({ told: true });
});

test("deleting an automation asks first", async ({ page }, testInfo) => {
  // Sessions require a second click to confirm. A scheduled task — which can send
  // messages and spend tokens on its own — is deleted by a single trash click.
  await boot(page);
  await go(page, "#/automations");
  await page.waitForTimeout(700);
  const del = page.getByRole("button", { name: /^delete /i }).first();
  test.skip(!(await del.count()), "no automation in this fixture state");
  const before = await page.locator(".sched-card, [class*=sched-card]").count();
  let dialogSeen = false;
  page.on("dialog", (d) => {
    dialogSeen = true;
    d.dismiss().catch(() => {});
  });
  await del.click();
  await page.waitForTimeout(900);
  const after = await page.locator(".sched-card, [class*=sched-card]").count();
  await shot(page, "deep-delete-automation");
  // "Asked" means any of the three: a native confirm, an undo, or the control
  // re-arming itself (the two-click pattern sessions use).
  const undo = await page.getByRole("button", { name: /undo/i }).count();
  const rearmed = await page.getByRole("button", { name: /confirm deleting/i }).count();
  await testInfo.attach("automation-delete", {
    body: JSON.stringify({ before, after, dialogSeen, undo, rearmed }, null, 2),
    contentType: "application/json",
  });
  expect(
    { asked: dialogSeen || undo > 0 || rearmed > 0, survived: after === before },
    `one click removed an automation with no confirmation and no undo (${before} → ${after} cards)`,
  ).toMatchObject({ asked: true, survived: true });
});
