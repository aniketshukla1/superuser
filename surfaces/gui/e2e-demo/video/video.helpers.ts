import path from "node:path";
import type { Locator, Page } from "@playwright/test";
import { HERO_MESSAGES } from "../demoData";

// The demo video: the real interface, driven the way a person drives it, over the same mock
// backend and invented demo data as the screenshots. Nothing in it is a mock-up: the turn
// arrives over the session socket in the engine's own event protocol (turn_start,
// tool_proposed, tool_finished, assistant_delta, permission_required, ...), so the steps,
// the chart, the streamed answer and the approval card are what the app draws for a real run.
//
// Frames come from Chrome's screencast: one JPEG per repaint, with its timestamp, written to
// <DEMO_OUT>/video. scripts/encode-video.swift turns them into the MP4. The cursor and the
// captions are drawn into the page for the recording, since a screencast has no pointer.
//
//   node node_modules/@playwright/test/cli.js test -c playwright.demo.config.ts video
//   swift ../../scripts/encode-video.swift ../../docs/assets/raw/video ../site/public/demo.mp4

export const OUT = process.env.DEMO_OUT || path.resolve(process.cwd(), "..", "..", "docs", "assets", "raw");
export const FRAMES = path.join(OUT, "video");
export const LIVE = "perf-live";

export const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

// ---- the turn, in the engine's event protocol ----

export const ANSWER = HERO_MESSAGES[1] as { items: { name: string; args: unknown }[]; content: string };
export const PREVIEW: Record<string, string[]> = {
  metrics_find: ['4 series: http_request_duration_seconds_bucket{service="checkout"}, by handler'],
  metrics_compare: [
    "/checkout/confirm p95 412 → 688 ms (+67%) at 14:02 · the other 4 handlers within normal variation",
    'Only version="2.41.0" moved · pods on 2.40.3 unchanged',
  ],
  metrics_alerts: ["No alerts firing or pending"],
  chart: ["Drew: p95 latency by handler (ms)"],
};
export const SLACK = {
  target: "slack:#checkout-team",
  text: "Checkout p95 is up 67% since the 2.41 deploy, only on /checkout/confirm (412 → 688 ms). Errors and CPU are flat. Likely cause: the new inline tax-service call. Full write-up in Superuser.",
};

export type Send = (type: string, data?: Record<string, unknown>) => void;

export async function stream(send: Send, text: string, every = 38) {
  const words = text.split(/(\s+)/);
  for (let i = 0; i < words.length; i += 4) {
    send("assistant_delta", { text: words.slice(i, i + 4).join("") });
    await sleep(every);
  }
}

export async function investigate(send: Send, input: string) {
  send("turn_start", { input });
  await sleep(700);
  const seen: Record<string, number> = {};
  for (const step of ANSWER.items) {
    send("tool_proposed", { name: step.name, arguments: step.args });
    await sleep(step.name === "chart" ? 600 : step.name === "metrics_compare" ? 1500 : 1000);
    const n = (seen[step.name] = (seen[step.name] ?? -1) + 1);
    send("tool_finished", { name: step.name, status: "ok", result_preview: PREVIEW[step.name]?.[n] ?? "ok" });
    await sleep(250);
  }
  await stream(send, ANSWER.content);
  send("assistant_message", { text: ANSWER.content });
  send("turn_done");
}

export async function propose(send: Send, input: string) {
  send("turn_start", { input });
  await sleep(600);
  await stream(send, "Here's the message. It goes to Slack, so it needs your yes:");
  send("tool_proposed", { name: "send_message", arguments: SLACK });
  send("permission_required", { name: "send_message", arguments: SLACK, reason: "", category: "messaging" });
}

export async function posted(send: Send) {
  send("tool_finished", { name: "send_message", status: "ok", result_preview: "Posted to #checkout-team" });
  await sleep(300);
  await stream(send, "Posted to #checkout-team.");
  send("assistant_message", { text: "Posted to #checkout-team." });
  send("turn_done");
}

// ---- the pointer and the captions, drawn into the page ----

export async function overlays(page: Page) {
  await page.evaluate(() => {
    const cursor = document.createElement("div");
    cursor.id = "demo-cursor";
    cursor.innerHTML =
      '<svg width="26" height="26" viewBox="0 0 24 24"><path d="M5 2.5l13.5 8.2-6.1 1.3-3.1 5.9z" fill="#fff" stroke="#0b0d0f" stroke-width="1.3" stroke-linejoin="round"/></svg>';
    cursor.style.cssText =
      "position:fixed;left:0;top:0;z-index:2147483647;pointer-events:none;will-change:transform;" +
      "transform:translate(1180px,760px);filter:drop-shadow(0 2px 4px rgba(0,0,0,.55))";
    document.body.appendChild(cursor);

    const cap = document.createElement("div");
    cap.id = "demo-caption";
    cap.style.cssText =
      "position:fixed;left:50%;bottom:26px;z-index:2147483646;pointer-events:none;max-width:78%;" +
      "transform:translate(-50%,8px);opacity:0;transition:opacity .35s ease,transform .35s ease;" +
      "padding:12px 22px;border-radius:12px;background:rgba(14,16,18,.96);border:1px solid #2c3237;" +
      "color:#e8eaed;font:500 18px/25px Inter,system-ui,sans-serif;text-align:center;box-shadow:0 14px 44px rgba(0,0,0,.55)";
    document.body.appendChild(cap);
  });
}

export async function caption(page: Page, text: string | null) {
  await page.evaluate((t) => {
    const cap = document.getElementById("demo-caption")!;
    if (t) cap.textContent = t;
    cap.style.opacity = t ? "1" : "0";
    cap.style.transform = t ? "translate(-50%,0)" : "translate(-50%,8px)";
  }, text);
}

export async function pointAt(page: Page, target: Locator, ms = 700) {
  const box = await target.boundingBox();
  if (!box) throw new Error("nothing to point at");
  const x = box.x + Math.min(box.width / 2, 60);
  const y = box.y + box.height / 2;
  await page.evaluate(
    ([x, y, ms]) => {
      const c = document.getElementById("demo-cursor")!;
      c.style.transition = `transform ${ms}ms cubic-bezier(.22,.61,.36,1)`;
      c.style.transform = `translate(${x - 5}px,${y - 3}px)`;
    },
    [x, y, ms] as const,
  );
  await sleep(ms + 60);
  return { x, y };
}

export async function clickOn(page: Page, target: Locator, ms = 700) {
  const { x, y } = await pointAt(page, target, ms);
  await page.evaluate(
    ([x, y]) => {
      const r = document.createElement("div");
      r.style.cssText =
        `position:fixed;left:${x - 16}px;top:${y - 16}px;width:32px;height:32px;border-radius:50%;` +
        "border:2px solid #8a8cf0;z-index:2147483645;pointer-events:none;opacity:.9;" +
        "transition:transform .45s ease-out,opacity .45s ease-out";
      document.body.appendChild(r);
      requestAnimationFrame(() => {
        r.style.transform = "scale(1.8)";
        r.style.opacity = "0";
      });
      setTimeout(() => r.remove(), 500);
    },
    [x, y] as const,
  );
  await target.click();
  await sleep(250);
}

export async function endCard(page: Page) {
  await page.evaluate(() => {
    const card = document.createElement("div");
    card.style.cssText =
      "position:fixed;inset:0;z-index:2147483647;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:18px;" +
      "background:radial-gradient(900px 520px at 70% 20%, rgba(138,140,240,.16), transparent 62%), #0b0d0f;color:#e8eaed;" +
      "opacity:0;transition:opacity .6s ease;font-family:Inter,system-ui,sans-serif";
    card.innerHTML =
      '<div style="font:500 30px/1 \'JetBrains Mono\',monospace;display:flex;align-items:center;gap:12px">' +
      '<span style="color:#8a8cf0">#</span><span style="display:inline-block;width:.42em;height:.92em;background:#e8eaed;border-radius:1px;margin-left:-6px"></span>' +
      "<span>superuser</span></div>" +
      '<div style="font-size:54px;font-weight:600;letter-spacing:-.028em">The AI you can tell everything.</div>' +
      '<div style="font-size:21px;color:#8b9299">Runs on your Mac. Asks before it acts. Shows you everything that left.</div>' +
      '<div style="margin-top:10px;font:400 17px/1 \'JetBrains Mono\',monospace;color:#8a8cf0">aniketshukla1.github.io/superuser</div>';
    document.body.appendChild(card);
    requestAnimationFrame(() => (card.style.opacity = "1"));
  });
}
