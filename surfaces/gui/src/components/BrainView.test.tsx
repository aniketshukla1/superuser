import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { BrainView } from "./BrainView";

// The three memory stores are separate surfaces, not one scroll — a section must
// show its own store and nothing else. These assert the split holds, because the
// failure mode is silent: mount the wrong child and the page still renders.

function stubFetch(routes: { match: string; json: any; ok?: boolean }[]) {
  const fn = vi.fn(async (url: string) => {
    for (const r of routes) {
      if (String(url).includes(r.match)) {
        return { ok: r.ok ?? true, json: async () => r.json } as Response;
      }
    }
    return { ok: true, json: async () => ({}) } as Response;
  });
  vi.stubGlobal("fetch", fn);
}

const MEMORY = {
  memories: [
    { filename: "a.md", title: "Deploy runbook", category: "GitHub", date: "2026-08-01", tags: [], origin: "external" },
  ],
};
const JOURNAL = { core_entries: 4, core: "", dailies: [], dreams: [], evolutions: [] };
const KNOWLEDGE = { pages: [{ slug: "postgres", title: "Postgres", category: "Concepts", body: "", updated_at: "2026-08-01", links: [] }] };

const stubAll = () =>
  stubFetch([
    { match: "/v1/memory", json: MEMORY },
    { match: "/v1/journal", json: JOURNAL },
    { match: "/v1/knowledge", json: KNOWLEDGE },
  ]);

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("BrainView", () => {
  it("lands on Memories and shows the vault, not the other two stores", async () => {
    stubAll();
    render(<BrainView />);

    expect(await screen.findByText("Deploy runbook")).toBeTruthy();
    expect(screen.getByPlaceholderText("Search memories…")).toBeTruthy();
    // The timeline and the topic wiki are siblings now; neither may render here.
    expect(screen.queryByRole("button", { name: "Consolidate now" })).toBeNull();
    expect(screen.queryByPlaceholderText("Search pages…")).toBeNull();
  });

  it("Timeline shows the journal", async () => {
    stubAll();
    render(<BrainView />);

    fireEvent.click(screen.getByTestId("brain-tab-timeline"));
    expect(await screen.findByRole("button", { name: "Consolidate now" })).toBeTruthy();
    expect(screen.queryByPlaceholderText("Search memories…")).toBeNull();
  });

  it("names the section once: the selected tab, not a second heading inside it", async () => {
    stubAll();
    render(<BrainView />);

    fireEvent.click(screen.getByTestId("brain-tab-timeline"));
    await screen.findByRole("button", { name: "Consolidate now" });
    expect(screen.queryByText("Memory journal")).toBeNull();
    expect(screen.getByTestId("brain-tab-timeline").getAttribute("aria-current")).toBe("page");
  });

  it("Knowledge shows the topic pages", async () => {
    stubAll();
    render(<BrainView />);

    fireEvent.click(screen.getByTestId("brain-tab-knowledge"));
    expect(await screen.findByText("Postgres")).toBeTruthy();
    expect(screen.queryByPlaceholderText("Search memories…")).toBeNull();
  });

  it("opens on the section it is asked for", async () => {
    stubAll();
    render(<BrainView initialSection="knowledge" />);

    expect(await screen.findByText("Postgres")).toBeTruthy();
  });

  it("says what has changed in the vault since it was last looked at", async () => {
    // The ledger has been derivable since the vault became a git repo and was
    // readable only over HTTP — the same complaint this plan opened with about
    // memory living in a settings tab.
    stubFetch([
      { match: "/v1/ledger", json: { initialized: true, snapshots: 12, sources: [
        { source: "github", added: 3, modified: 1, removed: 0, total: 117, summary: "github: 3 new, 1 changed." },
        { source: "notion", added: 0, modified: 0, removed: 0, total: 1, summary: "notion: nothing new." },
      ] } },
      { match: "/v1/memory", json: MEMORY },
      { match: "/v1/journal", json: JOURNAL },
      { match: "/v1/knowledge", json: KNOWLEDGE },
    ]);
    render(<BrainView />);

    const strip = await screen.findByTestId("ledger-strip");
    expect(strip.textContent).toContain("12 snapshots");
    expect(strip.textContent).toContain("github: 3 new, 1 changed");
    // A source with nothing new is noise on a strip about what changed.
    expect(strip.textContent).not.toContain("notion");
  });

  it("shows no ledger at all until the first sync creates one", async () => {
    // Not an error state: the repository is created by the first sync, so an
    // uninitialised ledger is the ordinary condition of a fresh vault.
    stubAll();
    render(<BrainView />);

    expect(await screen.findByText("Deploy runbook")).toBeTruthy();
    expect(screen.queryByTestId("ledger-strip")).toBeNull();
  });

  it("says where a memory's words came from, and treats an unlabelled one as outside", async () => {
    stubFetch([
      { match: "/v1/memory/a.md", json: { ok: true, memory: { ...MEMORY.memories[0], body: "b" } } },
      { match: "/v1/memory", json: MEMORY },
      { match: "/v1/journal", json: JOURNAL },
      { match: "/v1/knowledge", json: KNOWLEDGE },
    ]);
    render(<BrainView />);

    fireEvent.click(await screen.findByText("Deploy runbook"));
    expect(await screen.findByTestId("mem-origin-external")).toBeTruthy();
    expect(screen.queryByTestId("mem-origin-internal")).toBeNull();
  });
});
