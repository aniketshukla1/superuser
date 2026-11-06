import path from "node:path";
import type { Page } from "@playwright/test";
import { OUT, SLOW, type Send, sleep, stream } from "./video.helpers";

// The assistants film (the README's and the site's launch video): Product Performance traces a
// slowdown, Talent Scout leaves out everyone already in the ATS, and the Inbox holds what waits
// for a yes. Same real UI, same mock backend and invented data as demoVideo.spec.ts.

export const FILM = path.join(OUT, "film");
export const TALENT_LIVE = "talent-live";

// Invented people, invented projects: nothing here is anybody real.
const named = ["Mira Castellanos", "Jun Okafor", "Lena Varga", "Tomás Ibarra", "Priya Raman", "Sam Whitlock"];
const candidates = (n: number) => Array.from({ length: n }, (_, i) => ({ name: named[i % named.length] }));

const SCOUT_STEPS: { name: string; args: unknown; preview: string; ms: number }[] = [
  { name: "web_search", args: { query: "kubernetes operator maintainers on call" }, preview: "26 results: operator repositories, conference speakers", ms: 900 },
  { name: "candidates_check", args: { candidates: candidates(23) }, preview: "3 already in your ATS (same email) — left out · 20 new", ms: 1300 },
  { name: "candidates_shortlist", args: { role: "Senior SRE", candidates: candidates(20) }, preview: "Shortlisted 20 for “Senior SRE”", ms: 700 },
];

const SCOUT_ANSWER = [
  "**20 operator maintainers, none already in your ATS.** Three people the search found are in your ATS export, so they're left out.",
  "",
  "- **Mira Castellanos** — maintains a Postgres backup operator, four years of releases",
  "- **Jun Okafor** — wrote the autoscaler for an open-source CI runner",
  "- **Lena Varga** — co-maintains a Kafka operator; on-call rotation in its docs",
  "",
  "Evidence for every must-have and a draft in your voice for each: `outputs/shortlist.md`. Nobody has been contacted.",
].join("\n");

export async function scout(send: Send, input: string) {
  send("turn_start", { input });
  await sleep(600);
  for (const step of SCOUT_STEPS) {
    send("tool_proposed", { name: step.name, arguments: step.args });
    await sleep(step.ms);
    send("tool_finished", { name: step.name, status: "ok", result_preview: step.preview });
    await sleep(200);
  }
  await stream(send, SCOUT_ANSWER, 30);
  send("assistant_message", { text: SCOUT_ANSWER });
  send("turn_done");
}

/** Mark a moment in the manifest, so the edit and the score can find it. */
export const mark = (manifest: [string, number][], name: string) => manifest.push([`mark:${name}`, Date.now() / 1000]);

const JOBS: [string, string][] = [
  ["Email Assistant", "drafts replies that wait for your yes"],
  ["Expense Manager", "receipts in, totals that add up"],
  ["Chief of Staff", "what came in, mapped to your priorities"],
  ["Bug Reproduction", "reproduces bugs on staging, never production"],
  ["Product Performance", "finds the series that moved"],
  ["Talent Scout", "candidates, none already in your ATS"],
];

/** The last card: the six jobs, and where to get it. Carbon: matte black, ice white. It dips to
 *  black first and brings the words in after, so no frame is a double exposure of two screens. */
export async function jobsCard(page: Page) {
  await page.evaluate(([jobs, slow]) => {
    const t = (s: number) => `${s * slow}s`;
    const card = document.createElement("div");
    card.style.cssText =
      "position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;" +
      "background:radial-gradient(900px 520px at 50% 0%, rgba(214,228,255,.10), transparent 62%), #0d0d0d;color:#eef1f5;" +
      `opacity:0;transition:opacity ${t(0.28)} ease;font-family:Inter,system-ui,sans-serif`;
    const rows = jobs
      .map(
        ([n, j], i) =>
          `<div class="job" style="display:flex;gap:10px;align-items:baseline;opacity:0;transform:translateY(6px);` +
          `transition:opacity ${t(0.35)} ease ${t(0.55 + i * 0.09)},transform ${t(0.35)} ease ${t(0.55 + i * 0.09)}">` +
          `<span style="font-weight:600;font-size:19px;color:#eef1f5;white-space:nowrap">${n}</span>` +
          `<span style="font-size:17px;color:#8f969e">${j}</span></div>`,
      )
      .join("");
    card.innerHTML =
      `<div class="words" style="display:flex;flex-direction:column;align-items:center;gap:22px;opacity:0;transition:opacity ${t(0.4)} ease ${t(0.3)}">` +
      '<div style="font:500 22px/1 \'JetBrains Mono\',monospace;color:#cfd9e6;letter-spacing:.02em"># superuser</div>' +
      '<div style="font-size:50px;font-weight:600;letter-spacing:-.028em">Six assistants. Six whole jobs.</div>' +
      `<div style="display:grid;grid-template-columns:1fr 1fr;gap:12px 46px;margin:6px 0 4px">${rows}</div>` +
      '<div style="font-size:19px;color:#a9b0b8">On your Mac. Asks before it acts.</div>' +
      '<div style="font:400 16px/1 \'JetBrains Mono\',monospace;color:#cfd9e6">aniketshukla1.github.io/superuser</div></div>';
    document.body.appendChild(card);
    requestAnimationFrame(() => {
      card.style.opacity = "1";
      card.querySelector<HTMLElement>(".words")!.style.opacity = "1";
      card.querySelectorAll<HTMLElement>(".job").forEach((el) => {
        el.style.opacity = "1";
        el.style.transform = "none";
      });
    });
  }, [JOBS, SLOW] as const);
}
