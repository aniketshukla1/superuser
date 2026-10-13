// Session-screen cleanup (§22): the contextual top-left cluster ([sidebar][+][search], rendered
// ONLY while the sidebar is collapsed), the facts subtitle (who and where — fixed facts
// replacing the locked-model pill and the topbar About-persona button), and the composer row.
import { expect } from "@playwright/test";
import { test } from "./fixtures";

test("top-left cluster renders only while the sidebar is collapsed", async ({ page }) => {
  await page.goto("/");

  // Expanded sidebar owns those actions — no duplicate cluster.
  await expect(page.locator(".sidebar")).toBeVisible();
  await expect(page.getByTestId("topbar-cluster")).toHaveCount(0);

  // Collapse → the cluster appears with all three actions; the floating reveal button does NOT
  // double up on the session surface (the cluster's sidebar button replaces it).
  await page.keyboard.press("Meta+b");
  const cluster = page.getByTestId("topbar-cluster");
  await expect(cluster).toBeVisible();
  await expect(cluster.getByRole("button", { name: "Show sidebar" })).toBeVisible();
  await expect(cluster.getByRole("button", { name: "New session" })).toBeVisible();
  await expect(cluster.getByRole("button", { name: "Search" })).toBeVisible();
  await expect(page.locator(".nav-reveal-btn")).toHaveCount(0);

  // The cluster's search opens the command-palette overlay.
  await cluster.getByRole("button", { name: "Search" }).click();
  await expect(page.getByPlaceholder("Search chats")).toBeVisible();
  await page.keyboard.press("Escape");

  // The cluster's sidebar button docks the nav back — and the cluster leaves with it.
  await cluster.getByRole("button", { name: "Show sidebar" }).click();
  await expect(page.locator(".app")).not.toHaveClass(/nav-collapsed/);
  await expect(page.getByTestId("topbar-cluster")).toHaveCount(0);
});

test("facts subtitle: who, never the model; only what the heading isn't saying; inert", async ({
  page,
}) => {
  await page.goto("/");

  // Fresh-ish (boot-resumed, no rendered history): no subtitle, no old About-persona button —
  // and the model is a live PICKER in the composer.
  const sub = page.getByTestId("session-subtitle");
  await expect(sub).toHaveCount(0);
  await expect(page.getByRole("button", { name: "About this persona" })).toHaveCount(0);
  await expect(page.getByTestId("model-chip")).toContainText("Claude Opus 4.8");

  // First turn. The header names who and where, not which model (owner ask 2026-08-24): the
  // model stays on the composer's chip, where it can also be changed. An unnamed chat's
  // heading already IS the assistant, so there is nothing for a subtitle to add.
  const box = page.getByTestId("composer-input");
  await box.fill("hello");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText(/Echo: hello/)).toBeVisible();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Assistant");
  await expect(sub).toHaveCount(0);
  await expect(page.getByTestId("model-chip")).toContainText("Claude Opus 4.8");

  // Name the chat on its heading: the heading becomes the name, and the subtitle says who.
  await page.getByTestId("title-button").click();
  await page.getByTestId("title-input").fill("Launch prep");
  await page.getByTestId("title-input").press("Enter");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Launch prep");
  await expect(sub).toHaveText("Assistant");

  // A plain fact line, not a button to the persona page.
  await sub.click();
  await expect(page.getByRole("button", { name: "Back", exact: true })).toHaveCount(0);
});

test("composer keeps + add · Mode · send; folder and branch chips are gone", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByText("Draft the launch note").first().click();

  // The 2026-08-26 rebuild named "+" for what it does ("Add to this message") and put the
  // model, effort and context controls on the same row, right of these.
  await expect(page.getByRole("button", { name: "Add to this message" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Mode", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Send" })).toBeVisible();
  // The folder/roots popover trigger and the standalone Inbox control left the composer (§22).
  await expect(page.getByTitle(/director(y|ies) the agent can use/)).toHaveCount(0);
  await expect(page.getByTitle("Inbox routing")).toHaveCount(0);
  await expect(page.locator(".wschip")).toHaveCount(0);
  await expect(page.locator(".wsbranch")).toHaveCount(0);
});
