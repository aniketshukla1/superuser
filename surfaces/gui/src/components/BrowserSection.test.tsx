import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { BrowserSection } from "./BrowserSection";

// Settings ▸ Browser: the switch says what it turns on; a missing Chrome is said before
// anyone relies on it; Sign out of everything never happens without the confirmation that
// names what it deletes; and an assistant without a browser is sent to where it gets one.

const computer = (over: any = {}) => ({
  persona: "assistant",
  name: "Assistant",
  granted: true,
  running: false,
  url: "",
  title: "",
  control: { holder: "assistant", since: 0, requested: false },
  has_profile: true,
  viewers: 0,
  last_used: 0,
  ...over,
});

let posts: { url: string; body: any }[] = [];

function stub(state: any) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const reply = (v: any) => ({ ok: true, json: async () => v }) as Response;
      if (init?.method === "POST") {
        posts.push({ url: String(url), body: JSON.parse(String(init.body)) });
        return reply({ ok: true });
      }
      return reply(state);
    }),
  );
}

beforeEach(() => {
  posts = [];
  vi.stubGlobal("WebSocket", class {
    static OPEN = 1;
    readyState = 1;
    send() {}
    close() {}
  } as any);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

const base = { enabled: true, allow_loopback: false, headed: false, chrome_found: true, local_only: false };

describe("Settings ▸ Browser", () => {
  it("is off until switched on, and says so when there is no Chrome to use", async () => {
    stub({ ...base, enabled: false, chrome_found: false, computers: [] });
    render(<BrowserSection />);
    await screen.findByTestId("browser-no-chrome");
    expect(screen.queryByTestId("browser-loopback")).toBeNull();
    fireEvent.click(screen.getByTestId("browser-enabled"));
    await waitFor(() => expect(posts).toEqual([{ url: expect.stringContaining("/v1/settings/computer"), body: { browser_enabled: true } }]));
  });

  it("signs out of everything only after the confirmation", async () => {
    stub({ ...base, computers: [computer()] });
    render(<BrowserSection />);
    fireEvent.click(await screen.findByTestId("browser-reset-assistant"));
    const dialog = screen.getByTestId("browser-reset-confirm");
    expect(dialog.textContent).toContain("can’t be undone");
    expect(posts).toEqual([]);
    fireEvent.click(screen.getByTestId("browser-reset-go"));
    await waitFor(() => expect(posts.map((p) => p.url)).toEqual([expect.stringContaining("/v1/computers/assistant/reset")]));
  });

  it("opens a site for signing in and hands over the live view", async () => {
    stub({ ...base, computers: [computer()] });
    render(<BrowserSection />);
    fireEvent.click(await screen.findByTestId("browser-signin-assistant"));
    fireEvent.change(screen.getByTestId("browser-signin-url-assistant"), { target: { value: "mail.example.com" } });
    fireEvent.click(screen.getByText("Open it for me"));
    await waitFor(() => expect(posts[0]).toEqual({ url: expect.stringContaining("/v1/computers/assistant/open"), body: { url: "mail.example.com" } }));
    await screen.findByTestId("live-view");
  });

  it("says what an assistant is waiting for", async () => {
    stub({ ...base, computers: [computer({ running: true, control: { holder: "assistant", since: 0, requested: true, reason: "Sign in to the bank" } })] });
    render(<BrowserSection />);
    expect((await screen.findByTestId("browser-row-assistant")).textContent).toContain("Needs you: Sign in to the bank");
  });

  it("gives an assistant a browser here, and takes it away here", async () => {
    stub({ ...base, computers: [computer({ persona: "writer", name: "Writer", granted: false }), computer()] });
    render(<BrowserSection />);
    fireEvent.click(await screen.findByTestId("browser-grant-writer"));
    await waitFor(() => expect(posts[0]).toEqual({ url: expect.stringContaining("/v1/personas/writer"), body: { browser: true } }));
    fireEvent.click(screen.getByTestId("browser-revoke-assistant"));
    await waitFor(() => expect(posts[1]).toEqual({ url: expect.stringContaining("/v1/personas/assistant"), body: { browser: false } }));
  });
});
