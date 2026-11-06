import type { Page } from "@playwright/test";
import { test, boot, go, shot, watchErrors } from "./audit";

// LOCAL-ONLY (design-pass worktree): screenshots of every tab of every surface, with the Brain
// stores filled in so their layouts can be judged. Not for committing.

const JOURNAL = {
  core: "## About you\n- Prefers short answers with the command first.\n- Works on a Tauri + React desktop app.\n\n## Ongoing\n- A design pass on the GUI, in its own branch.",
  core_entries: 3,
  dailies: [
    { date: "2026-09-14", body: "**You:** can we fix the inbox cards?\n\n**Assistant:** Yes. The headline now names the tool and the argument sits in a preview block." },
    { date: "2026-09-13", body: "**You:** the automations list says `~7:00 AM`.\n\n**Assistant:** It reads *about 7:00 am* now." },
  ],
  recaps: [{ date: "2026-09-14", body: "Worked through inbox card copy; settled on **Allow once**." }],
  dreams: [],
  evolutions: [],
  nightly_enabled: true,
  evolution_enabled: false,
  can_undo: false,
  wiki: {
    patterns: [
      { id: "changelog", title: "Release skips the changelog", kind: "failure", seen: 3, first_seen: "2026-08-20", updated: "2026-09-10", body: "The release skill never tags the changelog, so 0.4.1 shipped without one." },
    ],
    impacts: [],
    log: "",
  },
};

const KNOWLEDGE = {
  pages: [
    { slug: "postgres", title: "Postgres", category: "infrastructure", body: "# Postgres\n\nThe primary store for the **session index**. Runs locally on port 5432.\n\n- Backed up nightly\n- Vacuumed weekly\n\nSee [[Status Brief]] for the migration plan.", updated_at: "2026-09-12T10:00:00Z", links: ["Status Brief", "Kubernetes"] },
    { slug: "status-note", title: "status-note", category: "projects", body: "Weekly status, written after each planning call.", updated_at: "2026-09-10T10:00:00Z", links: [] },
    { slug: "brief", title: "Status Brief", category: "projects", body: "The migration moves sessions off SQLite in two steps.", updated_at: "2026-09-09T10:00:00Z", links: ["Postgres"] },
    { slug: "release-checklist.md", title: "release-checklist.md", category: "projects", body: "1. Tag\n2. Changelog\n3. Notarize", updated_at: "2026-09-01T10:00:00Z", links: [] },
  ],
};

const PROFILE = {
  form: { name: "Sam", call_me: "", role: "Staff engineer", about: "", tone: "Direct", interests: "", likes: "", dislikes: "", goals: "" },
  facets: [
    { class: "style", key: "answer_length", value: "Short answers, command first", state: "active", score: 0.8, cue: "explicit", observations: 4, last_seen: 1757900000, evidence: "keep it short, put the command first", pinned: false, from_form: false },
    { class: "tooling", key: "editor", value: "Neovim", state: "provisional", score: 0.4, cue: "behavioral", observations: 2, last_seen: 1757800000, evidence: "opened the file in nvim", pinned: false, from_form: false },
    { class: "identity", key: "role", value: "Staff engineer", state: "active", score: 1, cue: "explicit", observations: 1, last_seen: 1757900000, evidence: "", pinned: false, from_form: true },
  ],
  forgotten: [],
  learning_enabled: true,
  path: "/Users/sam/Library/Application Support/Superuser/memory/PROFILE.md",
};

async function stores(page: Page) {
  await page.route("**/v1/journal", (r) => r.fulfill({ json: JOURNAL }));
  await page.route("**/v1/knowledge**", (r) => r.fulfill({ json: KNOWLEDGE }));
  await page.route("**/v1/profile", (r) => r.fulfill({ json: PROFILE }));
}

/** Every tab in the page's header tab row, shot one by one. Surfaces without one are shot as-is. */
async function everyTab(page: Page, prefix: string) {
  await shot(page, prefix);
  const tabs = page.locator("nav.page-tabs button");
  const n = await tabs.count();
  for (let i = 0; i < n; i++) {
    const t = tabs.nth(i);
    const label = ((await t.innerText()).split("\n")[0] || `tab${i}`).trim().toLowerCase().replace(/[^a-z0-9]+/g, "-");
    await t.click();
    await page.waitForTimeout(350);
    await shot(page, `${prefix}--${label}`);
  }
}

const SURFACES = [
  ["brain", "#/brain"],
  ["automations", "#/automations"],
  ["inbox", "#/inbox"],
  ["connectors", "#/connectors"],
  ["privacy", "#/activity"],
  ["settings", "#/settings"],
] as const;

for (const theme of ["light", "dark"] as const) {
  test(`design pass tabs @ ${theme}`, async ({ page }) => {
    const errors = watchErrors(page);
    await stores(page);
    if (theme === "dark") await page.addInitScript(() => localStorage.setItem("superuser.theme", "dark"));
    await page.setViewportSize({ width: 1100, height: 760 });
    await boot(page);
    for (const [name, hash] of SURFACES) {
      await go(page, hash);
      await everyTab(page, `dp-${theme}-${name}`);
    }
    if (errors.length) console.log(`[${theme}] errors:\n${errors.join("\n")}`);
  });
}

test("design pass: sidebar menus", async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await boot(page);
  await go(page, "#/activity");
  await shot(page, "dp-nav-privacy-active");
  await page.getByTestId("nav-more").click();
  await page.waitForTimeout(200);
  await shot(page, "dp-nav-more-open");
  await page.keyboard.press("Escape");
  await page.getByTestId("account-row").click();
  await page.waitForTimeout(200);
  await shot(page, "dp-nav-account-open");
});

test("design pass: the chat and its composer menus", async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 760 });
  await boot(page);
  await page.getByTestId("composer-input").fill("what changed in the release notes?");
  await shot(page, "dp-chat");

  await page.getByTestId("mode-chip").click();
  await page.waitForTimeout(200);
  await shot(page, "dp-chat-mode-menu");
  await page.getByTestId("mode-auto").click();
  await page.waitForTimeout(150);
  await shot(page, "dp-chat-mode-bypass-confirm");
  await page.keyboard.press("Escape");

  await page.getByTestId("context-ring").click();
  await page.waitForTimeout(300);
  await shot(page, "dp-chat-usage");
  await page.keyboard.press("Escape");

  await page.getByTestId("composer-input").fill("/");
  await page.waitForTimeout(300);
  await shot(page, "dp-chat-slash");
});

test("design pass: Brain at the minimum window", async ({ page }) => {
  await stores(page);
  await page.setViewportSize({ width: 880, height: 600 });
  await boot(page);
  await go(page, "#/brain");
  await everyTab(page, "dp-min-brain");
});
