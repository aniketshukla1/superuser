import { test, expect, boot, go, shot, SURFACES } from "../audit";
import { emptyEveryList, openPalette, visibleText } from "./deep.helpers";


test("a mutation that fails is not reported as success", async ({ page }, testInfo) => {
  await boot(page);
  await go(page, "#/settings");
  await page.waitForTimeout(600);

  // Reads keep working; every write fails. A UI that flips optimistically and never
  // reconciles will show the new state while the backend still holds the old one.
  await page.route("**/v1/**", async (r) => {
    if (r.request().method() === "GET") return r.fallback();
    return r.fulfill({ status: 500, contentType: "application/json", body: '{"error":"nope"}' });
  });

  const toggle = page.locator('input[type="checkbox"], button[role="switch"]').first();
  const has = await toggle.count();
  test.skip(!has, "no toggle on Settings to exercise");
  const read = async () =>
    toggle.evaluate((el: any) =>
      el.tagName === "INPUT" ? String(el.checked) : String(el.getAttribute("aria-checked")),
    );
  const which = await toggle.evaluate((el: HTMLElement) =>
    (el.closest("label")?.textContent || el.getAttribute("aria-label") || el.getAttribute("title") || "?")
      .replace(/\s+/g, " ")
      .trim()
      .slice(0, 60),
  );
  const before = await read();
  await toggle.click();
  await page.waitForTimeout(1200);
  const after = await read();
  const text = await visibleText(page);
  const complains = /couldn'?t|could not|failed|error|not saved|try again/i.test(text);
  await testInfo.attach("mutation", {
    body: JSON.stringify({ which, before, after, complains }, null, 2),
    contentType: "application/json",
  });
  await shot(page, "deep-failed-mutation");
  expect(
    { changed: before !== after, complains },
    `"${which}" moved to the new position on a 500 and nothing told the user the save failed (${before} → ${after})`,
  ).toMatchObject({ complains: true });
});

test("the command palette has a keyboard shortcut", async ({ page }) => {
  // It is a command palette in every other respect — ⌘1–9 to pick a row, an `Esc`
  // hint printed in its own header — but nothing opens it from the keyboard.
  await boot(page);
  await page.keyboard.press("Meta+k");
  await page.waitForTimeout(400);
  const openedByCmdK = await page.getByPlaceholder(/search chats/i).count();
  await page.keyboard.press("Control+k");
  await page.waitForTimeout(400);
  const openedByCtrlK = await page.getByPlaceholder(/search chats/i).count();
  expect(
    { openedByCmdK, openedByCtrlK },
    "neither ⌘K nor Ctrl+K opens the search palette; it can only be reached with the mouse",
  ).toMatchObject({ openedByCmdK: 1 });
});

test("the search palette makes the app behind it inert", async ({ page }, testInfo) => {
  await boot(page);
  await openPalette(page);
  const facts = await page.evaluate(() => {
    const overlay = document.querySelector('[role="dialog"], .fixed.inset-0') as HTMLElement | null;
    const root = document.getElementById("root");
    const shell = root?.firstElementChild as HTMLElement | null;
    return {
      overlayFound: !!overlay,
      role: overlay?.getAttribute("role") ?? null,
      ariaModal: overlay?.getAttribute("aria-modal") ?? null,
      // With no aria-modal and no inert/aria-hidden, a screen reader still walks the
      // whole app behind the palette.
      backgroundHidden: shell?.hasAttribute("inert") || shell?.getAttribute("aria-hidden") === "true",
      backgroundFocusable: Array.from(
        document.querySelectorAll("button, a[href], input, textarea, [tabindex]:not([tabindex='-1'])"),
      ).filter((el) => !(el as HTMLElement).closest('[role="dialog"], .fixed.inset-0')).length,
    };
  });
  await testInfo.attach("palette", { body: JSON.stringify(facts, null, 2), contentType: "application/json" });
  expect(facts, `the palette is a plain div over a fully live app:\n${JSON.stringify(facts, null, 2)}`).toMatchObject({
    role: "dialog",
    ariaModal: "true",
    backgroundHidden: true,
  });
});

test("Escape still closes the palette after focus has moved", async ({ page }) => {
  // The Escape handler is `onKeyDown` on the overlay div, so it only fires while
  // focus is inside it. Tab escapes the overlay (no trap), which means the user can
  // reach a state where the keyboard cannot dismiss the thing covering the screen.
  await boot(page);
  await openPalette(page);
  await expect(page.getByPlaceholder(/search chats/i)).toBeVisible();
  for (let i = 0; i < 12; i++) await page.keyboard.press("Tab");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);
  await shot(page, "deep-palette-stuck");
  await expect(
    page.getByPlaceholder(/search chats/i),
    "after tabbing out of the palette, Escape no longer closes it — the only way out is the mouse",
  ).toHaveCount(0);
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Empty account — the first-run experience
// ─────────────────────────────────────────────────────────────────────────────

test("a brand-new account gets an empty state with a next step on every surface", async ({ page }, testInfo) => {
  await emptyEveryList(page);
  await boot(page);
  const seen: any[] = [];
  for (const s of SURFACES) {
    await go(page, s.hash);
    await page.waitForTimeout(700);
    await shot(page, `deep-firstrun-${s.name}`);
    const facts = await page.evaluate(() => {
      const main = (document.querySelector("main, .main, .conn, .audit, .set") as HTMLElement) || document.body;
      const text = (main.innerText || "").replace(/\s+/g, " ").trim();
      const actions = Array.from(main.querySelectorAll("button, a[href]")).filter((el) => {
        const r = (el as HTMLElement).getBoundingClientRect();
        return r.width > 0 && r.height > 0;
      }).length;
      return { chars: text.length, actions, sample: text.slice(0, 160) };
    });
    seen.push({ surface: s.name, ...facts });
  }
  await testInfo.attach("first-run", { body: JSON.stringify(seen, null, 2), contentType: "application/json" });
  const empty = seen.filter((s) => s.chars < 40);
  expect(empty, `surfaces with nothing to say to a new user:\n${JSON.stringify(seen, null, 2)}`).toEqual([]);
  const dead = seen.filter((s) => s.actions === 0);
  expect(dead, `empty surfaces with no action to take:\n${JSON.stringify(seen, null, 2)}`).toEqual([]);
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. Copy
// ─────────────────────────────────────────────────────────────────────────────

test("no placeholder, debug, or retired-brand text is visible anywhere", async ({ page }, testInfo) => {
  await boot(page);
  const hits: any[] = [];
  const BAD = [
    /BigBrain/i,
    /\bundefined\b/,
    /\bNaN\b/,
    /\[object Object\]/,
    /\bTODO\b/,
    /\bFIXME\b/,
    /lorem ipsum/i,
    /\bnull\b/,
  ];
  for (const s of SURFACES) {
    await go(page, s.hash);
    await page.waitForTimeout(500);
    const text = await visibleText(page);
    for (const re of BAD) {
      const m = text.match(re);
      if (!m) continue;
      const at = text.indexOf(m[0]);
      const context = text.slice(Math.max(0, at - 50), at + 60);
      // Filesystem paths come from the mock's own fixture data, not from product
      // copy — flagging them would make this probe report on itself.
      if (/[~/][\w.~/-]*$/.test(text.slice(Math.max(0, at - 12), at))) continue;
      hits.push({ surface: s.name, pattern: String(re), context });
    }
  }
  await testInfo.attach("copy", { body: JSON.stringify(hits, null, 2), contentType: "application/json" });
  expect(hits, `strings the user should never see:\n${JSON.stringify(hits, null, 2)}`).toEqual([]);
});
