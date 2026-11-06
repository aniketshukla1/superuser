import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { TalentSection } from "./TalentSection";

// Settings ▸ General ▸ Talent Scout — the ATS export, the outreach voice, and the shortlist.

type Call = { url: string; method: string; body: any };

const BASE = { applicants: "/Users/sam/ats.csv", people: 1284, error: null, voice: "", shortlisted: 0 };

function stubFetch(routes: { match: string; method?: string; json: object }[]) {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const method = (init?.method || "GET").toUpperCase();
      calls.push({ url, method, body: init?.body ? JSON.parse(String(init.body)) : undefined });
      const hit = routes.find((r) => url.includes(r.match) && (!r.method || r.method === method));
      return { ok: true, json: async () => hit?.json ?? {} } as Response;
    }),
  );
  return calls;
}

const exportLine = async () => (await screen.findByTestId("talent-export")).textContent;

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("TalentSection", () => {
  it("says which export it checks against, how many people it holds, or why it can't", async () => {
    stubFetch([{ match: "/v1/talent", method: "GET", json: BASE }]);
    render(<TalentSection />);
    expect(await exportLine()).toBe("/Users/sam/ats.csv — 1284 people.");
    cleanup();
    stubFetch([{ match: "/v1/talent", method: "GET", json: { ...BASE, applicants: "", people: null } }]);
    render(<TalentSection />);
    expect(await exportLine()).toContain("candidates aren't checked against your applicants");
    expect(screen.queryByText("Stop using it")).toBeNull();
    cleanup();
    stubFetch([{ match: "/v1/talent", method: "GET", json: { ...BASE, people: null, error: "Couldn't read your ATS export at /Users/sam/ats.csv." } }]);
    render(<TalentSection />);
    expect(await exportLine()).toContain("Couldn't read your ATS export");
  });

  it("saves the file the picker chose, and nothing when it was cancelled", async () => {
    const calls = stubFetch([
      { match: "/v1/talent/pick", method: "POST", json: { path: "/Users/sam/new.csv" } },
      { match: "/v1/talent", method: "POST", json: { ...BASE, ok: true } },
      { match: "/v1/talent", method: "GET", json: BASE },
    ]);
    render(<TalentSection />);
    fireEvent.click(await screen.findByText("Choose another CSV…"));
    await waitFor(() => {
      const save = calls.find((c) => c.method === "POST" && c.url.endsWith("/v1/talent"));
      expect(save?.body).toEqual({ talent_applicants: "/Users/sam/new.csv" });
    });
    expect((await screen.findByRole("status")).textContent).toBe("Saved.");
    cleanup();
    const quiet = stubFetch([
      { match: "/v1/talent/pick", method: "POST", json: { path: null } },
      { match: "/v1/talent", method: "GET", json: BASE },
    ]);
    render(<TalentSection />);
    fireEvent.click(await screen.findByText("Choose another CSV…"));
    await waitFor(() => expect(quiet.some((c) => c.url.endsWith("/v1/talent/pick"))).toBe(true));
    expect(quiet.some((c) => c.method === "POST" && c.url.endsWith("/v1/talent"))).toBe(false);
  });

  it("shows why a file was refused", async () => {
    stubFetch([
      { match: "/v1/talent/pick", method: "POST", json: { path: "/Users/sam/notes.csv" } },
      { match: "/v1/talent", method: "POST", json: { ok: false, error: "Your ATS export at /Users/sam/notes.csv: its first row has no name, email or profile column." } },
      { match: "/v1/talent", method: "GET", json: BASE },
    ]);
    render(<TalentSection />);
    fireEvent.click(await screen.findByText("Choose another CSV…"));
    expect((await screen.findByRole("status")).textContent).toContain("no name, email or profile column");
  });

  it("saves the voice sample, trimmed", async () => {
    const calls = stubFetch([
      { match: "/v1/talent", method: "POST", json: { ...BASE, ok: true } },
      { match: "/v1/talent", method: "GET", json: BASE },
    ]);
    render(<TalentSection />);
    fireEvent.change(await screen.findByLabelText("A message that sounds like you"), { target: { value: "  Hi Sam —  " } });
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(calls.find((c) => c.method === "POST")?.body).toEqual({ talent_voice: "Hi Sam —" }));
  });

  it("forgets the shortlist, and offers to only when there is one", async () => {
    stubFetch([
      { match: "/v1/talent/forget", method: "POST", json: { ok: true, forgotten: 3 } },
      { match: "/v1/talent", method: "GET", json: { ...BASE, shortlisted: 3 } },
    ]);
    render(<TalentSection />);
    fireEvent.click(await screen.findByText("Forget the 3 shortlisted"));
    expect((await screen.findByRole("status")).textContent).toBe("Forgot 3.");
    cleanup();
    stubFetch([{ match: "/v1/talent", method: "GET", json: BASE }]);
    render(<TalentSection />);
    await screen.findByTestId("talent-export");
    expect(screen.queryByText(/Forget the/)).toBeNull();
  });
});
