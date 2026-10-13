// What a new chat opens on (a6950e3, 2026-08-24): the workspace's own record — sessions,
// messages, tokens, streak, busiest hour, most-used model and assistant, a heatmap — not a
// greeting. It replaced the persona intro, its starter chips and the three template rows this
// file used to cover ("What should we produce?"), which said the same thing on every new chat.
//
// The default mock leaves /v1/stats empty — a first run — so every other spec still starts on
// a bare new chat; these tests serve it themselves.
import { expect, type Page } from "@playwright/test";
import { test } from "./fixtures";

// The shape compute_stats (api_server.rs) answers with.
const STATS = {
  sessions: 12,
  messages: 86,
  tokens: 245_000,
  priced_messages: 40,
  active_days: 5,
  current_streak: 3,
  longest_streak: 4,
  peak_hour: 10,
  favorite_model: "anthropic:claude-opus-4-8",
  favorite_agent: "cowork",
  models: [
    { model: "anthropic:claude-opus-4-8", sessions: 9, messages: 70, input: 180_000, output: 40_000, tokens: 220_000 },
    { model: "gpt-5.5", sessions: 3, messages: 16, input: 20_000, output: 5_000, tokens: 25_000 },
  ],
  agents: [
    { agent: "cowork", sessions: 9 },
    { agent: "ops", sessions: 3 },
  ],
  days: [],
};

const serveStats = (page: Page, body: unknown) =>
  page.route("**/v1/stats*", (route) =>
    route.fulfill({ contentType: "application/json", body: JSON.stringify(body) }),
  );

test("a new chat opens on what the workspace has done, not on a greeting", async ({ page }) => {
  await serveStats(page, STATS);
  await page.goto("/");

  const panel = page.getByTestId("workspace-stats");
  await expect(panel).toBeVisible();
  const card = (label: string) => panel.locator(".wstats-card").filter({ hasText: label });
  await expect(card("Sessions")).toContainText("12");
  await expect(card("Messages")).toContainText("86");
  await expect(card("Tokens")).toContainText("245.0k");
  await expect(card("Streak")).toContainText("3 days");
  await expect(card("Busiest hour")).toContainText("10 AM");
  await expect(card("Most used model")).toContainText("claude-opus-4-8");
  await expect(card("Most used assistant")).toContainText("Assistant");

  // The greeting and its template rows are gone.
  await expect(page.getByText("What should we produce?")).toHaveCount(0);
  await expect(page.locator(".task-card")).toHaveCount(0);

  // A range is a new question to the server, not a filter over the first answer.
  const week = page.waitForRequest((r) => r.url().includes("/v1/stats?range=7d"));
  await panel.getByRole("button", { name: "7d" }).click();
  await week;
  await expect(panel.getByRole("button", { name: "7d" })).toHaveAttribute("aria-pressed", "true");

  // Models: each model's share of the measured tokens.
  await panel.getByRole("tab", { name: "Models" }).click();
  await expect(panel).toContainText("89.8%");
  await expect(panel).toContainText("gpt-5.5");
  await expect(panel).toContainText("10.2%");

  // The first message replaces the panel with the conversation.
  const box = page.getByTestId("composer-input");
  await box.fill("hello");
  await box.press("Enter");
  await expect(page.getByText(/Echo: hello/)).toBeVisible();
  await expect(panel).toHaveCount(0);
});

test("a first run renders nothing — an empty dashboard is worse than the blank screen", async ({
  page,
}) => {
  await serveStats(page, { ...STATS, sessions: 0, messages: 0, tokens: 0, priced_messages: 0, models: [], agents: [] });
  const answered = page.waitForResponse((r) => r.url().includes("/v1/stats"));
  await page.goto("/");
  await answered;

  await expect(page.getByTestId("composer-input")).toBeVisible();
  await expect(page.getByTestId("workspace-stats")).toHaveCount(0);
  await expect(page.getByText("What should we produce?")).toHaveCount(0);
});
