import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { JournalSection } from "./JournalSection";

// The Patterns tab is the one view of the memory journal that survives an undo.
// Every other tab records what a review *changed*; this records what it learned —
// and putting a skill back must not take the reasoning with it. These assert the
// asymmetry is visible, because the failure mode is silent: the tab still renders
// when it is showing the wrong half of the story.

function stubJournal(json: any) {
  vi.stubGlobal("fetch", vi.fn(async () => ({ ok: true, json: async () => json }) as Response));
}

const BASE = {
  core: "",
  core_entries: 0,
  dailies: [],
  recaps: [],
  dreams: [],
  evolutions: [],
  nightly_enabled: true,
  evolution_enabled: true,
  can_undo: false,
};

const PATTERN = {
  id: "changelog-step",
  title: "Release skips the changelog",
  kind: "failure" as const,
  seen: 3,
  first_seen: "2026-08-20",
  updated: "2026-08-31",
  body: "The release skill never tags the changelog, so 0.4.1 shipped without one.",
};

const REJECTED = {
  at: "2026-08-31 10:00",
  target: "release",
  patterns: ["changelog-step"],
  verdict: "undone" as const,
  snapshot: "evo-01",
  note: "added a changelog step",
};

const open = async () => {
  render(<JournalSection />);
  fireEvent.click(await screen.findByText(/Patterns/));
};

describe("a partial journal payload", () => {
  it("renders the section instead of unmounting the page", async () => {
    stubJournal({ core: "Prefers short answers." });
    render(<JournalSection />);
    expect(await screen.findByRole("button", { name: "Consolidate now" })).toBeTruthy();
    expect(screen.getByText("Prefers short answers.")).toBeTruthy();
  });

  it("does not report a switch the payload never mentioned as off", async () => {
    stubJournal({});
    render(<JournalSection />);
    await screen.findByRole("button", { name: "Consolidate now" });
    expect(screen.queryByText("Nightly consolidation is off.")).toBeNull();
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("the Patterns tab", () => {
  it("shows what a review learned, and how often it has seen it", async () => {
    stubJournal({ ...BASE, wiki: { patterns: [PATTERN], impacts: [], log: "" } });
    await open();

    expect(screen.getByText("Release skips the changelog")).toBeTruthy();
    expect(screen.getByText("failure")).toBeTruthy();
    // Recurrence is what earns a skill change, so it has to be on the row rather
    // than buried in the body.
    expect(screen.getByText("seen 3×")).toBeTruthy();
    expect(screen.getByText(/never tags the changelog/)).toBeTruthy();
  });

  it("keeps a rejected change on screen so it is visibly not retried", async () => {
    stubJournal({ ...BASE, wiki: { patterns: [PATTERN], impacts: [REJECTED], log: "" } });
    await open();

    expect(screen.getByText("Changes you turned down")).toBeTruthy();
    expect(screen.getByText("release")).toBeTruthy();
    expect(screen.getByText("added a changelog step")).toBeTruthy();
  });

  it("does not show an accepted change as something you turned down", async () => {
    // An accepted change is already visible as the skill it produced. Listing it
    // here would read as a rejection of a change that is still in force.
    stubJournal({
      ...BASE,
      wiki: { patterns: [PATTERN], impacts: [{ ...REJECTED, verdict: "accepted" as const }], log: "" },
    });
    await open();

    expect(screen.queryByText("Changes you turned down")).toBeNull();
    expect(screen.getByText("Release skips the changelog")).toBeTruthy();
  });

  it("explains itself when nothing has been noticed yet", async () => {
    stubJournal({ ...BASE, wiki: { patterns: [], impacts: [], log: "" } });
    await open();
    expect(screen.getByText(/Nothing noticed yet/)).toBeTruthy();
  });

  it("survives a core built before the wiki layer existed", async () => {
    // `wiki` is absent from an older sidecar; the tab must render empty, not crash.
    stubJournal(BASE);
    await open();
    expect(screen.getByText(/Nothing noticed yet/)).toBeTruthy();
  });

  it("says so when self-learning is switched off, since nothing will accrue", async () => {
    stubJournal({ ...BASE, evolution_enabled: false, wiki: { patterns: [PATTERN], impacts: [], log: "" } });
    await open();
    expect(screen.getByText("Self-learning is off.")).toBeTruthy();
  });
});
