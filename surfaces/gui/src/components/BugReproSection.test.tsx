import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { BugReproSection } from "./BugReproSection";

// Settings ▸ General ▸ Bug Reproduction — the staging address, and what its browser still needs.

type Call = { url: string; method: string; body: any };

const READY = { staging: "https://staging.example.com", browser: true, on_this_mac: false, loopback: false, local_only: false };

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

const ready = async () => (await screen.findByTestId("repro-ready")).textContent;

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("BugReproSection", () => {
  it("without a staging address, says its browser opens nothing", async () => {
    stubFetch([{ match: "/v1/bug-repro", method: "GET", json: { ...READY, staging: "" } }]);
    render(<BugReproSection />);
    expect(await ready()).toContain("opens nothing");
  });

  it("names the one thing still missing, first thing first", async () => {
    stubFetch([{ match: "/v1/bug-repro", method: "GET", json: { ...READY, browser: false } }]);
    render(<BugReproSection />);
    expect(await ready()).toBe("Turn on the assistants' browser in Settings ▸ Browser.");
    cleanup();
    stubFetch([{ match: "/v1/bug-repro", method: "GET", json: { ...READY, staging: "http://localhost:3000", on_this_mac: true } }]);
    render(<BugReproSection />);
    expect(await ready()).toContain("allow this Mac's own dev servers");
    cleanup();
    stubFetch([{ match: "/v1/bug-repro", method: "GET", json: { ...READY, local_only: true } }]);
    render(<BugReproSection />);
    expect(await ready()).toContain("Local-only mode is on");
  });

  it("with everything in place, says so", async () => {
    stubFetch([{ match: "/v1/bug-repro", method: "GET", json: READY }]);
    render(<BugReproSection />);
    expect(await ready()).toContain("Ready");
    expect((screen.getByLabelText("Staging address") as HTMLInputElement).value).toBe("https://staging.example.com");
  });

  it("saves the staging address, trimmed", async () => {
    const calls = stubFetch([
      { match: "/v1/bug-repro", method: "POST", json: { ...READY, ok: true } },
      { match: "/v1/bug-repro", method: "GET", json: { ...READY, staging: "" } },
    ]);
    render(<BugReproSection />);
    fireEvent.change(await screen.findByLabelText("Staging address"), { target: { value: "  https://staging.example.com  " } });
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST");
      expect(post?.body).toEqual({ repro_staging: "https://staging.example.com" });
    });
    expect((await screen.findByRole("status")).textContent).toBe("Saved.");
  });

  it("shows why an address was refused", async () => {
    stubFetch([
      {
        match: "/v1/bug-repro",
        method: "POST",
        json: { ok: false, error: "“169.254.169.254” is a cloud-metadata, link-local or reserved address, which the assistant's browser never reaches." },
      },
      { match: "/v1/bug-repro", method: "GET", json: { ...READY, staging: "" } },
    ]);
    render(<BugReproSection />);
    fireEvent.change(await screen.findByLabelText("Staging address"), { target: { value: "http://169.254.169.254" } });
    fireEvent.click(screen.getByText("Save"));
    expect((await screen.findByRole("status")).textContent).toContain("never reaches");
  });
});
