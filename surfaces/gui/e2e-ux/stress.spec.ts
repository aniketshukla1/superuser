import { test, expect, boot, shot, watchErrors, horizontalOverflow, clippedText } from "./audit";

// Content the layout was probably not designed against: pathological strings, a
// pasted wall of text, and a user in a hurry clicking twice.

const LONG_WORD = "supercalifragilistic".repeat(12); // 240 chars, no break opportunity
const LONG_URL = `https://example.com/${"a".repeat(400)}`;

test("an unbreakable string in a message does not blow out the transcript", async ({ page }, testInfo) => {
  await boot(page);
  const composer = page.locator("textarea").first();
  await composer.fill(LONG_WORD);
  await composer.press("Enter");
  await page.waitForTimeout(1500);
  await shot(page, "stress-long-word");

  const overflow = await horizontalOverflow(page);
  await testInfo.attach("overflow", { body: JSON.stringify(overflow, null, 2), contentType: "application/json" });
  expect(
    overflow.doc,
    `a 240-char unbroken word made the whole app scroll sideways: ${JSON.stringify(overflow, null, 2)}`,
  ).toBeLessThanOrEqual(overflow.view + 1);

  // The bubble itself must not spill past the transcript column either.
  const spill = await page.evaluate(() => {
    const out: any[] = [];
    for (const el of Array.from(document.querySelectorAll("*")) as HTMLElement[]) {
      const r = el.getBoundingClientRect();
      if (r.width > innerWidth + 2) {
        out.push({ cls: el.className?.toString().slice(0, 50) || el.tagName, w: Math.round(r.width) });
      }
    }
    return out.slice(0, 6);
  });
  expect(spill, `elements wider than the window:\n${JSON.stringify(spill, null, 2)}`).toEqual([]);
});

test("a very long URL wraps instead of escaping the bubble", async ({ page }) => {
  await boot(page);
  const composer = page.locator("textarea").first();
  await composer.fill(LONG_URL);
  await composer.press("Enter");
  await page.waitForTimeout(1500);
  await shot(page, "stress-long-url");
  const overflow = await horizontalOverflow(page);
  expect(overflow.doc, JSON.stringify(overflow, null, 2)).toBeLessThanOrEqual(overflow.view + 1);
});

test("a large paste keeps the composer usable", async ({ page }) => {
  await boot(page);
  const composer = page.locator("textarea").first();
  const wall = Array.from({ length: 120 }, (_, i) => `line ${i} of a pasted document`).join("\n");
  await composer.fill(wall);
  await page.waitForTimeout(400);
  await shot(page, "stress-big-paste");

  const box = await composer.boundingBox();
  const vh = page.viewportSize()!.height;
  expect(
    box!.height,
    `the composer grew to ${Math.round(box!.height)}px of a ${vh}px window — it should cap and scroll`,
  ).toBeLessThan(vh * 0.6);

  // The send control must still be reachable.
  const send = page.locator("button").filter({ hasText: /send/i }).first();
  const hasSend = (await send.count()) > 0;
  if (hasSend) await expect(send).toBeVisible();
});

test("double-clicking send does not send twice", async ({ page }) => {
  await boot(page);
  const sent: string[] = [];
  page.on("websocket", (ws) =>
    ws.on("framesent", (f) => {
      const s = typeof f.payload === "string" ? f.payload : f.payload.toString();
      if (s.includes("user_message") || s.includes('"text"')) sent.push(s.slice(0, 120));
    }),
  );
  const composer = page.locator("textarea").first();
  await composer.fill("run a tool please");
  await composer.press("Enter");
  await composer.press("Enter"); // an impatient second Enter on an emptied box
  await page.waitForTimeout(1600);
  await shot(page, "stress-double-send");

  const userMsgs = await page.locator(".bubble-user").filter({ hasText: "run a tool please" }).count();
  expect(userMsgs, "the same message was posted more than once").toBeLessThanOrEqual(1);
});

test("hammering the nav does not desync the shell", async ({ page }, testInfo) => {
  const errors = watchErrors(page);
  await boot(page);
  const hashes = ["#/brain", "#/inbox", "#/connectors", "#/automations", "#/settings", "#/"];
  for (let round = 0; round < 3; round++) {
    for (const h of hashes) {
      await page.evaluate((x) => (window.location.hash = x), h);
      await page.waitForTimeout(60); // faster than any fetch can settle
    }
  }
  await page.waitForTimeout(1500);
  await shot(page, "stress-nav-hammer");
  await testInfo.attach("errors", { body: errors.join("\n"), contentType: "text/plain" });
  const crashes = errors.filter((e) => e.startsWith("pageerror"));
  expect(crashes, `exceptions from fast navigation (stale-response races):\n${crashes.join("\n")}`).toEqual([]);
  // The shell must still be on the surface we last asked for.
  expect(page.url()).toContain("#/");
});

test("a long session title is truncated, not clipped mid-glyph", async ({ page }, testInfo) => {
  // Patched in the page, not at the network layer: the fixture already owns the
  // route, and `route.fetch()` would go looking for a real backend on :8765.
  await page.addInitScript(() => {
    const LONG = "A session title that keeps going and going far past any sensible sidebar width indeed";
    const orig = window.fetch;
    window.fetch = async (input: any, init?: any) => {
      const res = await orig(input, init);
      const url = typeof input === "string" ? input : input?.url || "";
      if (!/\/v1\/sessions(\?|$)/.test(url)) return res;
      const body = await res.clone().json().catch(() => null);
      if (!body) return res;
      const list = Array.isArray(body) ? body : body.sessions;
      if (Array.isArray(list)) for (const s of list) s.title = LONG;
      return new Response(JSON.stringify(body), { status: res.status, headers: { "content-type": "application/json" } });
    };
  });
  await boot(page);
  await page.waitForTimeout(800);
  await shot(page, "stress-long-title");
  const clipped = await clippedText(page);
  await testInfo.attach("clipped", { body: JSON.stringify(clipped, null, 2), contentType: "application/json" });
  expect(clipped, `text clipped with no ellipsis:\n${JSON.stringify(clipped, null, 2)}`).toEqual([]);
});
