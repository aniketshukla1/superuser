import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { WatchesView } from "./WatchesView";

type Call = { url: string; method: string; body: any };

function stubFetch(routes: { match: string; method?: string; json: any }[]) {
  const calls: Call[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    const method = (init?.method || "GET").toUpperCase();
    calls.push({ url: String(url), method, body: init?.body ? JSON.parse(String(init.body)) : undefined });
    for (const r of routes) {
      if (String(url).includes(r.match) && (!r.method || r.method === method)) {
        return { ok: true, json: async () => r.json } as Response;
      }
    }
    return { ok: true, json: async () => ({}) } as Response;
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

const WATCH = {
  id: "watch-1",
  question: "anything urgent from a customer?",
  sources: ["gmail"],
  enabled: true,
  writes: false,
  created_at: 1786000000,
  last_tick: null,
};

const ACTIVITY = [
  {
    id: "a1",
    watch_id: "watch-1",
    question: "anything urgent from a customer?",
    at: Date.now() / 1000 - 120,
    decision: "skip",
    reason: "only a newsletter and a sale",
    evidence: "gmail: 2 new",
    note: null,
    ask_id: null,
  },
];

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("WatchesView", () => {
  it("lists watches and the decisions taken about them", async () => {
    stubFetch([{ match: "/v1/watches", method: "GET", json: { watches: [WATCH], activity: ACTIVITY, enabled: true } }]);
    render(<WatchesView />);

    expect(await screen.findByTestId("watch-row")).toBeTruthy();
    expect(screen.getByText("gmail · not looked yet")).toBeTruthy();
    // The activity line is the page's point: a decision, its evidence and its reason.
    expect(screen.getByText("Nothing to do")).toBeTruthy();
    expect(screen.getByText("gmail: 2 new")).toBeTruthy();
    expect(screen.getByText("only a newsletter and a sale")).toBeTruthy();
  });

  it("adds a watch with the sources typed beside it", async () => {
    const calls = stubFetch([
      { match: "/v1/watches", method: "GET", json: { watches: [], activity: [], enabled: true } },
      { match: "/v1/watches", method: "POST", json: { ok: true, watch: WATCH } },
    ]);
    render(<WatchesView />);
    await screen.findByTestId("watch-add");

    fireEvent.change(screen.getByTestId("watch-question"), { target: { value: "did any repo break?" } });
    fireEvent.change(screen.getByTestId("watch-sources"), { target: { value: "GitHub, slack " } });
    fireEvent.click(screen.getByTestId("watch-add"));

    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST");
      expect(post).toBeTruthy();
      expect(post!.body.question).toBe("did any repo break?");
      // Normalized, so "GitHub" and "github " both match the connector id.
      expect(post!.body.sources).toEqual(["github", "slack"]);
    });
  });

  it("says so when the loop is switched off, rather than looking idle", async () => {
    stubFetch([{ match: "/v1/watches", method: "GET", json: { watches: [WATCH], activity: [], enabled: false } }]);
    render(<WatchesView />);
    expect(await screen.findByText(/background loop is switched off/i)).toBeTruthy();
  });

  it("reports a manual check that found nothing in plain words", async () => {
    stubFetch([
      { match: "/v1/watches/tick", method: "POST", json: { evaluated: 0, quiet: 1, acted: 0, escalated: 0, failed: 0 } },
      { match: "/v1/watches", method: "GET", json: { watches: [WATCH], activity: [], enabled: true } },
    ]);
    render(<WatchesView />);
    fireEvent.click(await screen.findByText("Check now"));
    expect(await screen.findByText(/Nothing has changed since the last look/i)).toBeTruthy();
  });

  it("deleting takes two clicks", async () => {
    const calls = stubFetch([
      { match: "/v1/watches", method: "GET", json: { watches: [WATCH], activity: [], enabled: true } },
      { match: "/v1/watches/watch-1", method: "DELETE", json: { ok: true } },
    ]);
    render(<WatchesView />);
    const del = await screen.findByTestId("watch-delete");

    fireEvent.click(del);
    expect(calls.some((c) => c.method === "DELETE")).toBe(false);
    expect(screen.getByTestId("watch-delete").textContent).toContain("Delete?");
    fireEvent.click(screen.getByTestId("watch-delete"));
    await waitFor(() => expect(calls.some((c) => c.method === "DELETE")).toBe(true));
  });
});
