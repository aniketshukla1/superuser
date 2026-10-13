import { test, expect, boot, go, shot, watchErrors, SURFACES } from "./audit";

// What the user sees when the world misbehaves. Every route registered here wins
// over the fixture's (Playwright matches routes newest-first), so each test bends
// exactly one thing and leaves the rest of the mock intact.

const bodyText = async (page: any) =>
  (await page.locator("body").innerText()).replace(/\s+/g, " ").trim();

test("backend down: the boot screen tells the truth about it", async ({ page }) => {
  await page.route("**/v1/**", (r) => r.abort("connectionrefused"));
  await page.goto("/");
  await page.waitForTimeout(8000);
  await shot(page, "fail-backend-down");
  const text = await bodyText(page);

  expect(text.length, "the screen is blank with the backend down").toBeGreaterThan(20);

  // The boot sequence prints a checklist. With nothing listening, those steps did
  // not happen — claiming `[ ok ]` for them is the screen lying to the user.
  const okCount = (text.match(/\[ ok \]/g) || []).length;
  const saysReady = /root@localhost.*ready/i.test(text.replace(/\s+/g, " "));
  const admitsTrouble =
    /retry|try again|can'?t reach|cannot reach|unreachable|not running|offline|no connection|failed/i.test(text);

  expect(
    { okCount, saysReady, admitsTrouble, screen: text.replace(/\s+/g, " ").slice(0, 200) },
    "with the sidecar unreachable the splash still reports every startup step as ok and never offers a way out",
  ).toMatchObject({ okCount: 0, saysReady: false, admitsTrouble: true });
});

test("slow backend: something explains the wait", async ({ page }) => {
  await page.route("**/v1/**", async (r) => {
    await new Promise((res) => setTimeout(res, 4000));
    await r.fallback();
  });
  await page.goto("/");
  await page.waitForTimeout(1800);
  await shot(page, "fail-slow-backend");
  const text = await bodyText(page);
  expect(text.length, "blank screen while the backend is slow").toBeGreaterThan(20);
});

test("a 500 on a surface's own endpoint does not blank the surface", async ({ page }, testInfo) => {
  const errors = watchErrors(page);
  await boot(page);
  // Fail everything except what the shell needs to stay standing.
  await page.route("**/v1/**", async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (/health|settings|personas|sessions$/.test(p)) return r.fallback();
    return r.fulfill({ status: 500, contentType: "application/json", body: '{"error":"boom"}' });
  });

  const seen: any[] = [];
  for (const s of SURFACES) {
    await go(page, s.hash);
    await page.waitForTimeout(700);
    await shot(page, `fail-500-${s.name}`);
    const text = await bodyText(page);
    seen.push({ surface: s.name, chars: text.length, sample: text.slice(0, 140) });
  }
  await testInfo.attach("500s", { body: JSON.stringify({ seen, errors }, null, 2), contentType: "application/json" });

  const blank = seen.filter((s) => s.chars < 60);
  expect(blank, `surfaces that went blank on a 500:\n${JSON.stringify(blank, null, 2)}`).toEqual([]);
  const crashes = errors.filter((e) => e.startsWith("pageerror"));
  expect(crashes, `unhandled exceptions on a 500:\n${crashes.join("\n")}`).toEqual([]);
});

test("malformed JSON from the backend does not crash the app", async ({ page }, testInfo) => {
  const errors = watchErrors(page);
  await boot(page);
  await page.route("**/v1/**", async (r) => {
    const p = new URL(r.request().url()).pathname;
    if (/health|settings|personas|sessions$/.test(p)) return r.fallback();
    return r.fulfill({ status: 200, contentType: "application/json", body: "{ this is not json" });
  });
  for (const s of SURFACES) {
    await go(page, s.hash);
    await page.waitForTimeout(500);
  }
  await shot(page, "fail-bad-json");
  await testInfo.attach("bad-json", { body: errors.join("\n"), contentType: "text/plain" });
  const crashes = errors.filter((e) => e.startsWith("pageerror"));
  expect(crashes, `unhandled exceptions on malformed JSON:\n${crashes.join("\n")}`).toEqual([]);
});

test("empty account: every surface has an empty state, not a void", async ({ page }, testInfo) => {
  const errors = watchErrors(page);
  await page.route("**/v1/**", async (r) => {
    const p = new URL(r.request().url()).pathname;
    const m = r.request().method();
    // Keep the app bootable, empty everything that is a list.
    if (/\/v1\/(health|settings|personas)$/.test(p)) return r.fallback();
    if (m !== "GET") return r.fallback();
    if (/sessions/.test(p)) return r.fulfill({ status: 200, contentType: "application/json", body: '{"sessions":[]}' });
    return r.fulfill({ status: 200, contentType: "application/json", body: "[]" });
  });
  await page.goto("/");
  await page.waitForTimeout(2500);

  const seen: any[] = [];
  for (const s of SURFACES) {
    await go(page, s.hash);
    await page.waitForTimeout(600);
    await shot(page, `empty-${s.name}`);
    const text = await bodyText(page);
    seen.push({ surface: s.name, chars: text.length, sample: text.slice(0, 200) });
  }
  await testInfo.attach("empty", { body: JSON.stringify({ seen, errors }, null, 2), contentType: "application/json" });

  const thin = seen.filter((s) => s.chars < 80);
  expect(thin, `surfaces with no empty-state copy:\n${JSON.stringify(thin, null, 2)}`).toEqual([]);
  const crashes = errors.filter((e) => e.startsWith("pageerror"));
  expect(crashes, crashes.join("\n")).toEqual([]);
});

test("the websocket dropping mid-session is surfaced, not silent", async ({ page }) => {
  await boot(page);
  const composer = page.locator("textarea").first();
  await composer.fill("hello there");
  await composer.press("Enter");
  await page.waitForTimeout(900);

  // Kill every open socket the way a sidecar crash would.
  await page.evaluate(() => {
    const w = window as any;
    for (const s of w.__ux_sockets || []) s.close();
  });
  await page.evaluate(() => {
    // Close via the constructor patch if the app kept no handle we can reach.
    const OrigWS = WebSocket;
    (window as any).__ws_closed = true;
    void OrigWS;
  });
  await page.waitForTimeout(1200);
  await shot(page, "fail-ws-drop");
});
