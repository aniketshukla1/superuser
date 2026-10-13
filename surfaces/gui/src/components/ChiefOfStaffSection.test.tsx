import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ChiefOfStaffSection } from "./ChiefOfStaffSection";

// Settings ▸ General ▸ Chief of Staff — the priorities, what feeds the digest, and its routine.

type Call = { url: string; method: string; body: any };

const BASE = { priorities: "", sources: [] as { source: string; items: number }[], routine: false };

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

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("ChiefOfStaffSection", () => {
  it("says what feeds the digest, so an empty one can be told from a blind one", async () => {
    stubFetch([{ match: "/v1/chief-of-staff", method: "GET", json: { ...BASE, sources: [{ source: "Email", items: 12 }, { source: "Calendar", items: 4 }] } }]);
    render(<ChiefOfStaffSection />);
    expect((await screen.findByTestId("cos-sources")).textContent).toBe("Reading this week: Email (12), Calendar (4).");
  });

  it("with nothing coming in, points to where things come from", async () => {
    stubFetch([{ match: "/v1/chief-of-staff", method: "GET", json: BASE }]);
    render(<ChiefOfStaffSection />);
    expect((await screen.findByTestId("cos-sources")).textContent).toContain("Connect your email, calendar or other apps");
  });

  it("saves the priorities, trimmed", async () => {
    const calls = stubFetch([
      { match: "/v1/chief-of-staff", method: "POST", json: { ...BASE, priorities: "Ship billing", ok: true } },
      { match: "/v1/chief-of-staff", method: "GET", json: BASE },
    ]);
    render(<ChiefOfStaffSection />);
    fireEvent.change(await screen.findByLabelText("Your priorities"), { target: { value: "  Ship billing\nHire two SREs  " } });
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST" && c.url.endsWith("/v1/chief-of-staff"));
      expect(post?.body).toEqual({ cos_priorities: "Ship billing\nHire two SREs" });
    });
  });

  it("adds the morning digest once, then says it runs", async () => {
    stubFetch([
      { match: "/v1/chief-of-staff/routine", method: "POST", json: { ok: true, created: true } },
      { match: "/v1/chief-of-staff", method: "GET", json: BASE },
    ]);
    render(<ChiefOfStaffSection />);
    fireEvent.click(await screen.findByText("Add a morning digest"));
    expect((await screen.findByRole("status")).textContent).toContain("weekdays at 8:00");
    cleanup();
    stubFetch([{ match: "/v1/chief-of-staff", method: "GET", json: { ...BASE, routine: true } }]);
    render(<ChiefOfStaffSection />);
    expect(await screen.findByText("A digest runs weekdays at 8:00.")).toBeTruthy();
    expect(screen.queryByText("Add a morning digest")).toBeNull();
  });
});
