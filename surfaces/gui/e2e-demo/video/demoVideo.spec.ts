import path from "node:path";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { test } from "../../e2e/fixtures";
import { EGRESS, SESSIONS } from "../demoData";
import { dress } from "../dress";
import { FRAMES, LIVE, Send, caption, clickOn, endCard, investigate, overlays, pointAt, posted, propose, sleep } from "./video.helpers";

test.use({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1.5 });
// ---- the recording ----

test("demo video", async ({ page }) => {
  test.setTimeout(240_000);
  rmSync(FRAMES, { recursive: true, force: true });
  mkdirSync(FRAMES, { recursive: true });

  await page.addInitScript(() => localStorage.setItem("superuser.theme", "dark"));
  await page.goto("/");
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });

  // The hero conversation becomes an empty one, which the video fills in live; the Network page
  // starts with the Slack post the video is about to make.
  const egress = {
    ...EGRESS,
    requests: [
      { at: Math.floor(Date.now() / 1000), purpose: "slack", method: "POST", host: "slack.com", path: "/api/chat.postMessage", sent: 412, status: 200, error: null, blocked: false },
      ...EGRESS.requests,
    ],
  };
  await dress(page, {
    sessions: SESSIONS.map((s) => (s.pinned ? { ...s, session_id: LIVE, messages: 0 } : s)),
    egress,
  });
  await page.route(/\/v1\/egress\/local-only$/, (route) => route.fulfill({ json: { ok: true, local_only: true } }));
  await page.route(/\/v1\/sessions\/perf-live\/messages(\?.*)?$/, (route) => route.fulfill({ json: { messages: [] } }));
  await page.routeWebSocket(/\/ws\/session\//, (ws) => {
    const send: Send = (type, data = {}) => ws.send(JSON.stringify({ type, data }));
    send("ready");
    ws.onMessage((raw) => {
      const msg = JSON.parse(String(raw));
      if (msg.type === "user_message" && /slower/i.test(msg.text)) void investigate(send, msg.text);
      else if (msg.type === "user_message") void propose(send, msg.text);
      else if (msg.type === "approval") void posted(send);
    });
  });

  await page.goto(`/#/chat/${LIVE}`);
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await sleep(1500);
  await overlays(page);

  // Record every repaint, with the time it happened.
  const cdp = await page.context().newCDPSession(page);
  const manifest: [string, number][] = [];
  cdp.on("Page.screencastFrame", async ({ data, metadata, sessionId }) => {
    const file = `f-${String(manifest.length + 1).padStart(5, "0")}.jpg`;
    writeFileSync(path.join(FRAMES, file), Buffer.from(data, "base64"));
    manifest.push([file, metadata.timestamp ?? Date.now() / 1000]);
    await cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
  });
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 92, maxWidth: 1920, maxHeight: 1200, everyNthFrame: 1 });

  const composer = page.getByPlaceholder(/Type \/ for commands/);

  // 1. Ask.
  await caption(page, "Superuser: an AI coworker that runs on your Mac.");
  await sleep(2400);
  await clickOn(page, composer, 900);
  await caption(page, "Ask in plain words.");
  await page.keyboard.type("Checkout got slower since yesterday's 2.41 release. Find where, with evidence.", { delay: 30 });
  await sleep(500);
  await page.keyboard.press("Enter");

  // 2. It works, and shows the work.
  await sleep(900);
  await caption(page, "It works in your real tools, and shows every step.");
  await page.locator(".chart").first().waitFor({ timeout: 30_000 });
  await caption(page, "A chart with its source. The facts kept apart from the guesses.");
  await page.getByText("I changed no alerts and no settings.").waitFor({ timeout: 30_000 });
  await sleep(2600);

  // 3. Anything that leaves asks first.
  await clickOn(page, composer, 800);
  await caption(page, null);
  await page.keyboard.type("Post a short summary to #checkout-team.", { delay: 30 });
  await sleep(400);
  await page.keyboard.press("Enter");
  const allow = page.getByRole("button", { name: "Allow once" }).last();
  await allow.waitFor({ timeout: 30_000 });
  await caption(page, "Anything that leaves your Mac waits for your yes.");
  await sleep(2200);
  await clickOn(page, allow, 900);
  await page.getByText("Posted to #checkout-team.").last().waitFor({ timeout: 30_000 });
  await sleep(1500);

  // 4. What left, and the switch that stops it.
  await caption(page, null);
  await clickOn(page, page.getByText("More", { exact: true }).first(), 900);
  await clickOn(page, page.getByRole("menuitem", { name: /Privacy/ }).first(), 600);
  await caption(page, "Every request that left your Mac, and who received it.");
  await sleep(2600);
  await clickOn(page, page.getByTestId("local-only-toggle"), 900);
  await caption(page, "Local-only mode: nothing leaves, and every refusal is written down.");
  await sleep(2800);

  // 5. The rules.
  await caption(page, null);
  await clickOn(page, page.getByTestId("settings-button").first(), 900);
  await clickOn(page, page.getByRole("button", { name: "Boundaries", exact: true }).first(), 700);
  await caption(page, "Your rules, enforced in code before every action.");
  await sleep(3000);

  // 6. What it knows.
  await caption(page, null);
  await clickOn(page, page.getByText("Brain", { exact: true }).first(), 900);
  await caption(page, "What it knows about you is markdown files on your disk.");
  await sleep(1000);
  await pointAt(page, page.getByText("Priorities for this week").first(), 900);
  await sleep(2400);

  // 7. The name.
  await caption(page, null);
  await endCard(page);
  await sleep(3600);

  await cdp.send("Page.stopScreencast");
  manifest.push(["end", Date.now() / 1000]);
  writeFileSync(path.join(FRAMES, "manifest.json"), JSON.stringify(manifest));
});
