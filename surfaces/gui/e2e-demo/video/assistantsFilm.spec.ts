import path from "node:path";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { test } from "../../e2e/fixtures";
import { SESSIONS } from "../demoData";
import { session } from "../demoData/conversations";
import { dress } from "../dress";
import { LIVE, SLOW, Send, caption, clickOn, investigate, overlays, pointAt, sleep } from "./video.helpers";
import { FILM, TALENT_LIVE, jobsCard, mark, scout } from "./assistantsFilm.helpers";

// The assistants film, filmed at 1920×1080: the real UI over the demo backend.
//
//   node node_modules/@playwright/test/cli.js test -c playwright.demo.config.ts assistantsFilm
//
// Frames and a manifest (with `mark:<scene>` entries) land in <DEMO_OUT>/film; the edit, the
// score and the encode happen outside the browser.

test.use({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1.5 });

test("assistants film", async ({ page }) => {
  test.setTimeout(600_000);
  rmSync(FILM, { recursive: true, force: true });
  mkdirSync(FILM, { recursive: true });

  await page.addInitScript(() => localStorage.setItem("superuser.theme", "dark"));
  await page.goto("/");
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 120_000 });
  const live = SESSIONS.map((s: any) => (s.pinned ? { ...s, session_id: LIVE, messages: 0 } : s));
  await dress(page, { sessions: [session(TALENT_LIVE, "Senior SRE — operator maintainers", "talent", 1), ...live] });
  for (const id of [LIVE, TALENT_LIVE]) {
    await page.route(new RegExp(`/v1/sessions/${id}/messages(\\?.*)?$`), (route) => route.fulfill({ json: { messages: [] } }));
  }
  await page.routeWebSocket(/\/ws\/session\//, (ws) => {
    const send: Send = (type, data = {}) => ws.send(JSON.stringify({ type, data }));
    send("ready");
    ws.onMessage((raw) => {
      const msg = JSON.parse(String(raw));
      if (msg.type !== "user_message") return;
      if (/slower/i.test(msg.text)) void investigate(send, msg.text);
      else void scout(send, msg.text);
    });
  });

  // A reload, not just the hash: the app reads its assistants once, at start.
  await page.goto(`/#/chat/${LIVE}`);
  await page.reload();
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 120_000 });
  await sleep(1500);
  // The activity panel only says "waiting for frames" in a demo: the conversation gets the width.
  await page.getByRole("button", { name: "Hide side panel" }).first().click();
  await sleep(400);
  await overlays(page);
  // Above the composer, so a caption never sits on what is being typed.
  await page.evaluate(() => (document.getElementById("demo-caption")!.style.bottom = "112px"));

  // Full-resolution screenshots in a loop, not the screencast: the screencast captures at CSS
  // pixels and drops the 1.5× ratio, which is the difference between 720p and 1080p.
  const cdp = await page.context().newCDPSession(page);
  const manifest: [string, number][] = [];
  let filming = true;
  const camera = (async () => {
    while (filming) {
      const at = Date.now() / 1000;
      const { data } = await cdp.send("Page.captureScreenshot", {
        format: "jpeg",
        quality: 90,
        optimizeForSpeed: true,
        clip: { x: 0, y: 0, width: 1280, height: 720, scale: 1.5 },
      });
      const file = `f-${String(manifest.filter(([f]) => f.startsWith("f-")).length + 1).padStart(5, "0")}.jpg`;
      writeFileSync(path.join(FILM, file), Buffer.from(data, "base64"));
      manifest.push([file, at]);
    }
  })();
  const composer = page.getByPlaceholder(/Type \/ for commands/);

  // 1. The hook: a slowdown, asked in plain words.
  mark(manifest, "hook");
  await caption(page, "Checkout got slower. Ask Product Performance.");
  await sleep(500);
  await clickOn(page, composer, 500);
  mark(manifest, "click");
  await page.keyboard.type("Checkout got slower since yesterday's 2.41 release. Find where.", { delay: 22 * SLOW });
  await page.keyboard.press("Enter");

  // 2. It finds the one series that moved.
  await sleep(500);
  mark(manifest, "perf");
  await caption(page, "It compares every series before and after the release, and finds the one that moved.");
  await page.locator(".chart").first().waitFor({ timeout: 120_000 });
  mark(manifest, "chart");
  await page.getByText("I changed no alerts and no settings.").waitFor({ timeout: 120_000 });
  mark(manifest, "answer");
  await sleep(1700);

  // 3. Talent Scout, and the ATS check that is code.
  await caption(page, null);
  mark(manifest, "talent");
  await clickOn(page, page.getByText("Talent Scout", { exact: true }).first(), 700);
  mark(manifest, "click");
  await clickOn(page, page.getByText("Senior SRE — operator maintainers").first(), 500);
  mark(manifest, "click");
  await caption(page, "Talent Scout finds candidates, and leaves out everyone already in your ATS.");
  await clickOn(page, composer, 500);
  mark(manifest, "click");
  await page.keyboard.type("Find 20 SREs who maintain a Kubernetes operator.", { delay: 22 * SLOW });
  await page.keyboard.press("Enter");
  await page.getByText("Nobody has been contacted.").waitFor({ timeout: 120_000 });
  mark(manifest, "answer");
  await sleep(500);
  // The check itself, opened on camera: the step that left three people out.
  await clickOn(page, page.getByText("3 steps").first(), 600);
  mark(manifest, "click");
  await sleep(1900);

  // 4. Whatever would leave waits for a yes.
  await caption(page, null);
  mark(manifest, "inbox");
  await clickOn(page, page.getByRole("button", { name: /^Inbox/ }).first(), 700);
  mark(manifest, "click");
  // The email's own Allow button mid-screen, clear of the caption below it.
  const allowEmail = page.getByRole("button", { name: "Allow once" }).nth(1);
  await allowEmail.evaluate((el) => el.scrollIntoView({ block: "center", behavior: "smooth" }));
  await caption(page, "Every email and post waits in your Inbox for a yes.");
  await sleep(500);
  await pointAt(page, allowEmail, 800);
  mark(manifest, "point");
  await sleep(2200);

  // 5. The six jobs.
  await caption(page, null);
  mark(manifest, "end");
  await jobsCard(page);
  await sleep(4400);

  filming = false;
  await camera;
  manifest.push(["end", Date.now() / 1000]);
  writeFileSync(path.join(FILM, "manifest.json"), JSON.stringify(manifest));
});
