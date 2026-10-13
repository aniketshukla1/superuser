import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { NetworkSection } from "./NetworkSection";

// The page that backs the product's central claim. What matters is that it is
// legible when there's nothing to show, honest about what it doesn't record, and
// that a refusal reads as a refusal rather than as a failed request.

const STATE = {
  requests: [
    {
      at: Date.now() / 1000,
      purpose: "github",
      method: "GET",
      host: "api.github.com",
      path: "/user/repos",
      sent: 0,
      status: 200,
      error: null,
      blocked: false,
    },
    {
      at: Date.now() / 1000,
      purpose: "model",
      method: "POST",
      host: "api.anthropic.com",
      path: "/v1/messages",
      sent: 4096,
      status: null,
      error: "Local-only mode is on",
      blocked: true,
    },
  ],
  hosts: [
    { host: "api.github.com", requests: 12, sent: 0, blocked: 0, last: Date.now() / 1000, purposes: ["github"] },
    { host: "api.anthropic.com", requests: 1, sent: 4096, blocked: 1, last: Date.now() / 1000, purposes: ["model"] },
  ],
  local_only: false,
};

function stubFetch(state: any = STATE) {
  const calls: { url: string; body: any }[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : null });
    if (String(url).includes("/v1/egress/local-only")) {
      return { ok: true, json: async () => ({ ok: true, local_only: true }) } as Response;
    }
    if (String(url).includes("/v1/egress/clear")) {
      return { ok: true, json: async () => ({ ok: true }) } as Response;
    }
    return { ok: true, json: async () => state } as Response;
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("NetworkSection", () => {
  it("leads with who has heard from you, busiest first", async () => {
    stubFetch();
    render(<NetworkSection />);

    const hosts = await screen.findAllByTestId("egress-host");
    expect(hosts[0].textContent).toContain("api.github.com");
    expect(hosts[0].textContent).toContain("github");
  });

  // 3.3 MB of prompts to a model provider sat in the same card, at the same weight, as a
  // page fetch that sent nothing. A host that answers model calls is marked as one.
  it("marks the hosts that receive your prompts", async () => {
    stubFetch();
    render(<NetworkSection />);
    const hosts = await screen.findAllByTestId("egress-host");
    const anthropic = hosts.find((h) => h.textContent?.includes("api.anthropic.com"))!;
    const github = hosts.find((h) => h.textContent?.includes("api.github.com"))!;
    expect(anthropic.textContent).toContain("receives your prompts");
    expect(github.textContent).not.toContain("receives your prompts");
  });

  it("asks before clearing the log, because it cannot be undone", async () => {
    const calls = stubFetch();
    render(<NetworkSection />);
    fireEvent.click(await screen.findByText("Clear log…"));
    expect(calls.some((c) => c.url.includes("/v1/egress/clear"))).toBe(false);
    fireEvent.click(screen.getByTestId("egress-clear-confirm"));
    await waitFor(() => expect(calls.some((c) => c.url.includes("/v1/egress/clear"))).toBe(true));
  });

  it("shows a refusal as a refusal, not as a failed request", async () => {
    // A blocked request never happened. Rendering it with a blank status would
    // read as "we tried and something went wrong", which is the opposite.
    stubFetch();
    render(<NetworkSection />);

    expect(await screen.findByText("refused")).toBeTruthy();
    expect(screen.getByText(/1 refused by local-only mode/)).toBeTruthy();
  });

  it("says why an empty log is empty, including what it never records", async () => {
    // "Nothing here" has to distinguish "nothing left the machine" from "we
    // aren't looking" — and the local-only calls to Ollama are the reason someone
    // would otherwise expect rows.
    stubFetch({ requests: [], hosts: [], local_only: false });
    render(<NetworkSection />);

    expect(await screen.findByText(/Nothing has left this machine/)).toBeTruthy();
    expect(screen.getByText(/never went anywhere/)).toBeTruthy();
  });

  it("carries the hard switch, next to the evidence of what it prevents", async () => {
    const calls = stubFetch();
    render(<NetworkSection />);

    const toggle = await screen.findByTestId("local-only-toggle");
    fireEvent.click(toggle);

    await waitFor(() => expect(calls.some((c) => c.url.includes("/v1/egress/local-only"))).toBe(true));
    expect(calls.find((c) => c.url.includes("local-only"))!.body).toEqual({ enabled: true });
    expect(await screen.findByText(/nothing may leave this machine/)).toBeTruthy();
  });
});
