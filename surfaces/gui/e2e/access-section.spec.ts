// The workspace panel's Access TAB (§32 — absorbs the §23 Session-settings drawer; the
// topbar row/glance machinery is retired). Contract: selecting the tab IS the disclosure —
// there is no header, no summary line and no second control to expand, and everything is
// edited inline in the panel (no overlay, no dialog).
// Fixture state: browser + slack + github connected/enabled (github is two_way WITHOUT
// channels — relay mentions, no subscriptions), gmail recommended-not-connected, one
// primary root → summary "Browser, Slack +1 · 1 folder".
import { expect } from "@playwright/test";
import { test } from "./fixtures";

test("no topbar opener; the Access tab shows everything at once and edits inline", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();

  // §32: the settings row/icon is gone from the topbar — the panel toggle is the one entry.
  await expect(page.getByRole("button", { name: "Open session settings" })).toHaveCount(0);
  await expect(page.getByTestId("session-settings-row")).toHaveCount(0);

  // The tab is the only door: no collapsed header, no summary line, and nothing left to
  // expand once you are here.
  await page.getByTestId("wp-tab-access").click();
  const section = page.getByTestId("access-section");
  await expect(section.getByTestId("access-toggle")).toHaveCount(0);
  await expect(section.getByTestId("access-summary")).toHaveCount(0);

  // Sources (per-session toggles), Recommended (with its reason) and Folders are all
  // present at once, inline in the panel; no dialog appears anywhere.
  const body = page.getByRole("region", { name: "Session access" });
  await expect(body.getByText("Sources")).toBeVisible();
  await expect(body.getByText("Slack", { exact: true })).toBeVisible();
  await expect(body.getByText("email context for morning summaries")).toBeVisible();
  await expect(body.getByTestId("drawer-directories").getByText("Temporary space")).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);

  // Channels is a chat capability, not a two_way one: Slack gets the drill-down, GitHub
  // (two_way via the relay, no channel semantics) must NOT (owner report 2026-07-13).
  await expect(body.getByRole("button", { name: /Channels ·/ })).toHaveCount(1);
  await expect(body.getByText("GitHub", { exact: true })).toBeVisible();
});

test("+ Add a source: full catalog on focus, filter as you type → connect-in-context; connected sources never match", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();
  await page.getByTestId("wp-tab-access").click();

  // Focusing the empty input shows the FULL catalog (FB-012) — every available connector
  // minus the already-connected three, before any typing.
  await page.getByTestId("access-add-source").click();
  const search = page.getByTestId("access-add-search");
  await expect(search).toBeFocused();
  const rows = page.locator('[data-testid^="access-add-"]:not([data-testid="access-add-search"])');
  await expect(rows).toHaveCount(9); // 12 in the catalog − browser/slack/github (connected)
  await expect(page.getByTestId("access-add-notion")).toBeVisible();

  // Already-connected sources don't match (Slack and GitHub are connected in fixtures)…
  await search.fill("slack");
  await expect(page.getByText("No match — see all on the Connectors page below.")).toBeVisible();
  await search.fill("github");
  await expect(page.getByText("No match — see all on the Connectors page below.")).toBeVisible();

  // …and clearing the query restores the full list ("filter as you type", not search-only).
  await search.fill("");
  await expect(rows).toHaveCount(9);

  // Capability aliases match too: "calendar" surfaces Outlook (title alone never would).
  await search.fill("calendar");
  await expect(page.getByTestId("access-add-outlook")).toBeVisible();

  // …the long tail does: Notion is in the catalog but neither connected nor recommended.
  await search.fill("notion");
  await page.getByTestId("access-add-notion").click();

  // Lands in the SAME connect-in-context child view the Recommended flow uses, with the
  // scope-semantics line; back returns to the Sources list.
  const body = page.getByRole("region", { name: "Session access" });
  await expect(body.getByText("Connecting makes Notion available to all your assistants", { exact: false })).toBeVisible();
  await expect(body.getByPlaceholder("ntn_…")).toBeVisible();
  await body.getByRole("button", { name: "Back to sources" }).click();
  await expect(body.getByText("Slack", { exact: true })).toBeVisible();
});

test("per-session mute round-trips", async ({ page }) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();
  await page.getByTestId("wp-tab-access").click();

  const body = page.getByRole("region", { name: "Session access" });
  // Muting Slack for this session persists: the fixture flips `enabled` on POST and the
  // section reloads, so the switch must come back off rather than springing back on. The
  // summary used to be what this asserted; the row itself now carries the state.
  const slack = body.getByTitle("Enabled for this session — tap to mute here").nth(1);
  await expect(slack).toHaveAttribute("aria-checked", "true");
  await slack.click();
  await expect(body.getByRole("switch").nth(1)).toHaveAttribute("aria-checked", "false");
});

test("Read/Write is offered for a source a message can leave through, and only for those", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();
  await page.getByTestId("wp-tab-access").click();

  const body = page.getByRole("region", { name: "Session access" });
  // Slack can be sent through, so it carries the choice…
  await expect(body.getByTestId("scope-read")).toHaveCount(1);
  await expect(body.getByTestId("scope-write")).toHaveCount(1);
  // …and starts at the reach it always had, rather than silently narrowed.
  await expect(body.getByTestId("scope-write")).toHaveAttribute("aria-pressed", "true");

  // Browser and GitHub cannot be sent through — GitHub despite being catalogued
  // two_way — so exactly one picker exists on the whole tab, not three.
  await expect(body.getByRole("group", { name: "What this session may do" })).toHaveCount(1);
});

test("narrowing a source to read round-trips", async ({ page }) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();
  await page.getByTestId("wp-tab-access").click();

  const body = page.getByRole("region", { name: "Session access" });
  await body.getByTestId("scope-read").click();
  // The fixture stores what was sent and the section reloads, so Read must hold
  // rather than springing back to Write.
  await expect(body.getByTestId("scope-read")).toHaveAttribute("aria-pressed", "true");
  await expect(body.getByTestId("scope-write")).toHaveAttribute("aria-pressed", "false");
});
