import {
  test,
  expect,
  boot,
  go,
  SURFACES,
  namelessControls,
  smallTargets,
  contrastFailures,
  tabWalk,
} from "./audit";

// Accessibility + keyboard-operability audit. Hand-rolled rather than axe-core so
// the audit adds no dependency to the app; the checks mirror the WCAG criteria a
// desktop app actually gets judged on.

test("every interactive control has an accessible name", async ({ page }, testInfo) => {
  await boot(page);
  const all: any[] = [];
  for (const s of SURFACES) {
    await go(page, s.hash);
    const bad = await namelessControls(page);
    if (bad.length) all.push({ surface: s.name, count: bad.length, controls: bad });
  }
  await testInfo.attach("nameless", { body: JSON.stringify(all, null, 2), contentType: "application/json" });
  expect(all, `controls with no accessible name:\n${JSON.stringify(all, null, 2)}`).toEqual([]);
});

test("hit targets meet the 24px minimum", async ({ page }, testInfo) => {
  await boot(page);
  const all: any[] = [];
  for (const s of SURFACES) {
    await go(page, s.hash);
    const bad = await smallTargets(page);
    if (bad.length) all.push({ surface: s.name, targets: bad });
  }
  await testInfo.attach("small-targets", { body: JSON.stringify(all, null, 2), contentType: "application/json" });
  expect(all, `sub-24px hit targets:\n${JSON.stringify(all, null, 2)}`).toEqual([]);
});

for (const theme of ["light", "dark"] as const) {
  test(`text contrast meets WCAG AA — ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((t) => localStorage.setItem("superuser.theme", t), theme);
    await boot(page);
    const all: any[] = [];
    for (const s of SURFACES) {
      await go(page, s.hash);
      const bad = await contrastFailures(page);
      if (bad.length) all.push({ surface: s.name, failures: bad });
    }
    await testInfo.attach(`contrast-${theme}`, {
      body: JSON.stringify(all, null, 2),
      contentType: "application/json",
    });
    expect(all, `contrast below AA (${theme}):\n${JSON.stringify(all, null, 2)}`).toEqual([]);
  });
}

test("keyboard focus is always visible and never lost", async ({ page }, testInfo) => {
  await boot(page);
  const seq = await tabWalk(page, 45);
  await testInfo.attach("tab-walk", { body: JSON.stringify(seq, null, 2), contentType: "application/json" });

  const landed = seq.filter((s) => s.tag !== "body");
  expect(landed.length, "Tab never moved focus off <body>").toBeGreaterThan(5);

  const noRing = landed.filter((s) => !s.ring);
  expect(
    noRing,
    `focused with no visible indicator (keyboard users cannot see where they are):\n${JSON.stringify(noRing, null, 2)}`,
  ).toEqual([]);

  const offscreen = landed.filter((s) => s.offscreen);
  expect(offscreen, `focus landed on something not visible:\n${JSON.stringify(offscreen, null, 2)}`).toEqual([]);
});

test("the search palette is a dialog, traps focus, and Escape closes it", async ({ page }, testInfo) => {
  await boot(page);
  // The palette opens on the Search affordance or ⌘K.
  const search = page.getByRole("button", { name: /search/i }).first();
  if (await search.count()) await search.click();
  else await page.keyboard.press("Meta+k");
  await page.waitForTimeout(350);

  // Addressed by its placeholder rather than "the first input on the page": the
  // palette is portalled to <body>, so it is last in DOM order, not first.
  const input = page.getByPlaceholder(/search chats/i);
  await expect(input, "the palette should focus its input on open").toBeFocused();

  // Collected in one shot — an early assertion would hide the rest of the picture.
  const dialog = await page.evaluate(() => {
    const el = document.querySelector('[role="dialog"]');
    return el ? { modal: el.getAttribute("aria-modal"), label: el.getAttribute("aria-label") } : null;
  });

  // The overlay is the fixed-inset layer holding the input, whether or not it is
  // labelled as a dialog.
  const overlaySel = '[role="dialog"], .fixed.inset-0';
  const escaped: any[] = [];
  // More presses than the palette has focusable rows, so a missing trap shows up.
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press("Tab");
    const outside = await page.evaluate((sel) => {
      const el = document.activeElement as HTMLElement | null;
      if (!el || el === document.body) return "body";
      return el.closest(sel) ? null : el.className?.toString().slice(0, 40) || el.tagName;
    }, overlaySel);
    if (outside) escaped.push({ step: i, focused: outside });
  }

  // Escape after focus has wandered: the handler lives on the overlay, so this is
  // the question that matters — can the user still dismiss it?
  await page.keyboard.press("Escape");
  await page.waitForTimeout(250);
  const stillOpenAfterTabbingOut = (await page.locator(overlaySel).filter({ has: page.locator("input") }).count()) > 0;

  const facts = { dialogRole: dialog, escapedFocus: escaped, stillOpenAfterTabbingOut };
  await testInfo.attach("palette", { body: JSON.stringify(facts, null, 2), contentType: "application/json" });

  expect(facts, `search-palette modal semantics:\n${JSON.stringify(facts, null, 2)}`).toEqual({
    dialogRole: { modal: "true", label: expect.any(String) },
    escapedFocus: [],
    stillOpenAfterTabbingOut: false,
  });
});

test("Escape closes the palette when focus never left it", async ({ page }) => {
  await boot(page);
  const search = page.getByRole("button", { name: /search/i }).first();
  if (await search.count()) await search.click();
  else await page.keyboard.press("Meta+k");
  await page.waitForTimeout(300);
  const input = page.getByPlaceholder(/search chats/i);
  await expect(input).toBeVisible();
  await page.keyboard.press("Escape");
  await page.waitForTimeout(250);
  await expect(input, "Escape from inside the palette should close it").toHaveCount(0);
});
