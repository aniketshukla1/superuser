import { test, expect, boot, go, shot, SURFACES } from "./audit";

// Pins down *why* the window goes white, and how far a single bad response
// propagates. Reported separately from resilience.spec so the cause travels with
// the symptom.

function trap(page: any) {
  const errs: { msg: string; stack: string }[] = [];
  page.on("pageerror", (e: any) => errs.push({ msg: e.message, stack: (e.stack || "").split("\n").slice(0, 4).join(" | ") }));
  page.on("console", (m: any) => {
    if (m.type() === "error") errs.push({ msg: `console: ${m.text().slice(0, 300)}`, stack: "" });
  });
  return errs;
}

const rootHtml = (page: any) => page.evaluate(() => document.getElementById("root")?.innerHTML.length ?? -1);

test("one 500 from one surface takes down the whole window", async ({ page }, testInfo) => {
  const errs = trap(page);
  await boot(page);
  expect(await rootHtml(page), "the app should be mounted before we break anything").toBeGreaterThan(1000);

  // Break exactly one endpoint — the Inbox list — and leave everything else alone.
  await page.route("**/v1/inbox**", (r) =>
    r.fulfill({ status: 500, contentType: "application/json", body: '{"error":"boom"}' }),
  );

  await go(page, "#/inbox");
  await page.waitForTimeout(900);
  await shot(page, "white-inbox-500");
  const afterInbox = await rootHtml(page);

  // Can the user get back out by navigating away?
  await go(page, "#/");
  await page.waitForTimeout(900);
  await shot(page, "white-after-nav-away");
  const afterNavAway = await rootHtml(page);

  await testInfo.attach("errors", { body: JSON.stringify(errs, null, 2), contentType: "application/json" });

  const crash = errs.find((e) => !e.msg.startsWith("console:"));
  const summary = `#root innerHTML: ${afterInbox} chars after the Inbox 500, ${afterNavAway} chars after navigating away.\ncrash: ${crash ? `${crash.msg} @ ${crash.stack}` : "none"}`;
  expect(afterInbox, `the Inbox 500 unmounted the window.\n${summary}`).toBeGreaterThan(1000);
  expect(afterNavAway, `navigating away did not bring the app back.\n${summary}`).toBeGreaterThan(1000);
  expect(crash?.msg ?? null, `unhandled render exception.\n${summary}`).toBeNull();
});

// `api.ts` unwraps 21 list endpoints as `(await res.json()).<key>`. Eighteen end in
// `?? []`; three do not — getPersonas (1082), getInbox (1492), getUnattended (1601).
// Personas load during boot, so that one decides whether the app starts at all.
test("a bad /v1/personas response leaves the app unable to start", async ({ page }) => {
  const errs = trap(page);
  await page.route("**/v1/personas", (r) =>
    r.fulfill({ status: 500, contentType: "application/json", body: '{"error":"boom"}' }),
  );
  await page.goto("/");
  await page.waitForTimeout(4000);
  await shot(page, "white-personas-500");
  const chars = await rootHtml(page);
  const crash = errs.find((e) => !e.msg.startsWith("console:"));
  expect(
    chars,
    `the app never mounted when /v1/personas failed (#root = ${chars} chars).\ncrash: ${crash?.msg} @ ${crash?.stack}`,
  ).toBeGreaterThan(1000);
});

// App itself calls getInbox during boot, so the same missing fallback decides
// whether a cold start survives a bad `/v1/inbox`.
test("a bad /v1/inbox response at cold start stops the app booting at all", async ({ page }) => {
  const errs = trap(page);
  await page.route("**/v1/inbox**", (r) =>
    r.fulfill({ status: 500, contentType: "application/json", body: '{"error":"boom"}' }),
  );
  await page.goto("/");
  await page.waitForTimeout(5000);
  await shot(page, "white-cold-start-inbox-500");
  const chars = await rootHtml(page);
  const crash = errs.find((e) => !e.msg.startsWith("console:"));
  expect(
    chars,
    `cold start with a failing /v1/inbox left #root at ${chars} chars.\ncrash: ${crash?.msg} @ ${crash?.stack}`,
  ).toBeGreaterThan(1000);
});

test("which surfaces survive an empty backend, one at a time", async ({ page }, testInfo) => {
  const results: any[] = [];
  const errs = trap(page);
  await boot(page);

  // One route, armed and disarmed through a flag — `unrouteAll` would also tear
  // down the fixture's own mock and leave the app with no backend at all.
  let empty = false;
  await page.route("**/v1/**", async (r) => {
    if (!empty || r.request().method() !== "GET") return r.fallback();
    const p = new URL(r.request().url()).pathname;
    if (/\/v1\/(health|settings|personas)$/.test(p)) return r.fallback();
    return r.fulfill({ status: 200, contentType: "application/json", body: "[]" });
  });

  for (const s of SURFACES) {
    const before = errs.length;
    empty = true;
    await go(page, s.hash);
    await page.waitForTimeout(800);
    const rootChars = await rootHtml(page);
    results.push({
      surface: s.name,
      rootChars,
      crash: errs.slice(before).find((e) => !e.msg.startsWith("console:"))?.msg?.slice(0, 120) ?? null,
    });
    empty = false;
    if (rootChars < 500) {
      // The tree is gone; remount so the next surface is measured from a live app.
      await page.reload();
      await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
      await page.waitForTimeout(500);
    }
  }
  await testInfo.attach("per-surface", { body: JSON.stringify(results, null, 2), contentType: "application/json" });
  const dead = results.filter((r) => r.rootChars < 500);
  expect(dead, `surfaces that blanked on an empty backend:\n${JSON.stringify(results, null, 2)}`).toEqual([]);
});
