import { test, expect, boot, go, shot } from "../audit";
import { visibleText } from "./deep.helpers";


// ─────────────────────────────────────────────────────────────────────────────
// 1. The shell at the size the product ships
// ─────────────────────────────────────────────────────────────────────────────

test("at 880x600 the three columns share the window instead of squeezing the chat", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 880, height: 600 });
  await boot(page);
  await page.locator("textarea").first().fill("ready to send");
  await page.waitForTimeout(300);
  await shot(page, "deep-min-shell");

  const geom = await page.evaluate(() => {
    const box = (sel: string) => {
      const el = document.querySelector(sel) as HTMLElement | null;
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return { x: Math.round(r.x), w: Math.round(r.width), right: Math.round(r.right) };
    };
    const ta = document.querySelector("textarea") as HTMLElement;
    const composerRow = ta?.closest("form") || ta?.parentElement?.parentElement || null;
    const send =
      (document.querySelector('button[aria-label="Send"], button[title*="Send"]') as HTMLElement) ||
      (composerRow?.querySelector("button:last-of-type") as HTMLElement);
    const sendRect = send?.getBoundingClientRect();
    const hit = sendRect
      ? document.elementFromPoint(sendRect.left + sendRect.width / 2, sendRect.top + sendRect.height / 2)
      : null;
    return {
      viewport: innerWidth,
      nav: box("nav, .nav, aside.side, .sidebar"),
      chat: box(".main-chat"),
      rail: box(".right-rail"),
      textarea: box("textarea"),
      send: sendRect
        ? { x: Math.round(sendRect.x), w: Math.round(sendRect.width), right: Math.round(sendRect.right) }
        : null,
      sendCoveredBy: hit ? (hit as HTMLElement).className?.toString().slice(0, 40) : null,
      sendReachable: !!hit && !!send && (hit === send || send.contains(hit) || hit.contains(send)),
    };
  });
  await testInfo.attach("geometry", { body: JSON.stringify(geom, null, 2), contentType: "application/json" });

  // 45 characters of prose at 14.5px is roughly 360px. Below that the transcript
  // stops being a reading column and becomes a gutter.
  expect(
    geom.chat?.w ?? 0,
    `the chat column is ${geom.chat?.w}px of an ${geom.viewport}px window:\n${JSON.stringify(geom, null, 2)}`,
  ).toBeGreaterThan(360);
  expect(
    geom.sendReachable,
    `the Send button is not the topmost element at its own centre:\n${JSON.stringify(geom, null, 2)}`,
  ).toBe(true);
});

test("the desktop brand row fits inside its sidebar", async ({ page }, testInfo) => {
  // The macOS build turns the sidebar's brand row into the window's title bar, and the
  // traffic lights float over its left end. Clearing them sideways costs 84px of a 240px
  // sidebar, and that over-subscribed the row twice while it also carried a wordmark: once
  // rendering "superuser bet", and again — after the sidebar narrowed and gained a search
  // button — rendering "superus" AND pushing search clean off the edge, where no mouse or
  // Tab could reach it.
  //
  // The wordmark is back, but only as a mark here: a container query hides its name until
  // the row has room, and the Home/Workspace switch is two icons. That is what keeps the
  // sideways inset affordable. The guard stays because the failure mode does not depend on
  // a wordmark — anything added to this row can push the last control past the edge, silently.
  //
  // `?overlay=1` is the dev flag that reproduces that layout in a browser (App.tsx:1303),
  // which is the only way to test it without a desktop shell.
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/?overlay=1");
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForTimeout(600);
  await shot(page, "deep-overlay-brand");

  const fit = await page.evaluate(() => {
    const row = document.querySelector(".brand") as HTMLElement | null;
    const search = document.querySelector('[data-testid="nav-search"]') as HTMLElement | null;
    const bar = document.querySelector(".sidebar") as HTMLElement | null;
    if (!row || !bar) return null;
    return {
      overlayActive: document.querySelector(".app")?.classList.contains("tauri-overlay") ?? false,
      needs: row.scrollWidth,
      has: row.clientWidth,
      searchFound: !!search,
      searchRight: search ? Math.round(search.getBoundingClientRect().right) : null,
      sidebarRight: Math.round(bar.getBoundingClientRect().right),
    };
  });
  await testInfo.attach("brand-row", { body: JSON.stringify(fit, null, 2), contentType: "application/json" });
  expect(fit?.overlayActive, "?overlay=1 did not put the app in the desktop layout").toBe(true);
  expect(
    fit!.needs,
    `the brand row needs ${fit!.needs}px of a ${fit!.has}px sidebar, so its right-hand end is sliced off:\n${JSON.stringify(fit, null, 2)}`,
  ).toBeLessThanOrEqual(fit!.has);
  // The load-bearing one, now that the row is only controls. `needs <= has` alone is a weak
  // check: anything on this row that can shrink absorbs the overflow and reports a row that
  // "fits" while a control sits outside the sidebar. A position check cannot be fooled that
  // way, so it is what the test turns on.
  expect(fit?.searchFound, "no search button on the brand row to check").toBe(true);
  expect(
    fit!.searchRight!,
    `the search button runs past the sidebar's edge (${fit!.searchRight} > ${fit!.sidebarRight}), so it is unreachable`,
  ).toBeLessThanOrEqual(fit!.sidebarRight);
});

test("200% text zoom reflows instead of scrolling sideways", async ({ page }) => {
  // WCAG 1.4.4 / 1.4.10: doubling text size must not require two-axis scrolling.
  await page.setViewportSize({ width: 1280, height: 800 });
  await boot(page);
  await page.evaluate(() => (document.documentElement.style.fontSize = "32px"));
  await page.waitForTimeout(600);
  await shot(page, "deep-zoom-200");
  const state = await page.evaluate(() => ({
    doc: document.documentElement.scrollWidth,
    view: document.documentElement.clientWidth,
    composerVisible: (() => {
      const ta = document.querySelector("textarea");
      if (!ta) return false;
      const r = ta.getBoundingClientRect();
      return r.width > 0 && r.top < innerHeight && r.bottom > 0;
    })(),
  }));
  expect(state.doc, `at 200% text size the app scrolls sideways: ${JSON.stringify(state)}`).toBeLessThanOrEqual(
    state.view + 1,
  );
  expect(state.composerVisible, `the composer left the viewport at 200% text size: ${JSON.stringify(state)}`).toBe(true);
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Failure that reaches the user
// ─────────────────────────────────────────────────────────────────────────────

test("a render crash is caught and offers a way back", async ({ page }) => {
  await boot(page);
  // The known crash path: a non-conforming /v1/inbox body. What matters here is not
  // that it throws — it is whether anything catches it.
  await page.route("**/v1/inbox**", (r) =>
    r.fulfill({ status: 500, contentType: "application/json", body: '{"error":"boom"}' }),
  );
  await go(page, "#/inbox");
  await page.waitForTimeout(1200);
  await shot(page, "deep-crash-boundary");
  const text = await visibleText(page);
  expect(
    text.length,
    `the window is blank after one bad response — there is no error boundary anywhere in the tree, so a throw in any surface unmounts the whole app (body text: ${text.length} chars)`,
  ).toBeGreaterThan(20);
});

test("a backend that never answers eventually says so", async ({ page }) => {
  // App.tsx retries health 40 × 500ms. This probe waits past that budget and asks
  // what the user is looking at when the retries run out.
  await page.route("**/v1/**", (r) => r.abort("connectionrefused"));
  await page.goto("/");
  await page.waitForTimeout(26_000);
  await shot(page, "deep-backend-never");
  const text = await visibleText(page);
  const admits = /can'?t reach|cannot reach|unreachable|not running|offline|no connection|failed|retry|try again/i.test(
    text,
  );
  const retryControl = await page
    .getByRole("button", { name: /retry|try again|reload|restart/i })
    .count();
  expect(
    { admits, retryControl, screen: text.slice(0, 240) },
    "after the health retries are exhausted the user is given neither an explanation nor a retry",
  ).toMatchObject({ admits: true });
});
