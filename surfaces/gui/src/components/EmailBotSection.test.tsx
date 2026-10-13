import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { EmailBotSection } from "./EmailBotSection";

// Settings ▸ General ▸ Answer email — the email bot's switch and sender list.

type Call = { url: string; method: string; body: any };

const BASE = {
  connected: true,
  address: "me@example.com",
  inbound: false,
  allowed: "",
  allowed_count: 0,
  live: false,
  problem: null,
  drafted_today: 0,
  daily_cap: 40,
};

function stubFetch(first: object, afterSave: object = first) {
  const calls: Call[] = [];
  let saved = false;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const method = (init?.method || "GET").toUpperCase();
      calls.push({ url, method, body: init?.body ? JSON.parse(String(init.body)) : undefined });
      if (method === "POST") saved = true;
      const json = method === "POST" ? { ...afterSave, ok: true } : saved ? afterSave : first;
      return { ok: true, json: async () => json } as Response;
    }),
  );
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("EmailBotSection", () => {
  it("asks for the email connection first when there is none", async () => {
    stubFetch({ ...BASE, connected: false, address: "" });
    render(<EmailBotSection />);
    expect(await screen.findByText(/Connect your email first/)).toBeTruthy();
    expect(screen.queryByRole("switch")).toBeNull();
  });

  it("switching it on saves the switch and the list, and says it is live", async () => {
    const calls = stubFetch(BASE, { ...BASE, inbound: true, allowed: "ada@example.com", allowed_count: 1, live: true });
    render(<EmailBotSection />);
    fireEvent.change(await screen.findByLabelText("Senders to answer"), { target: { value: " ada@example.com " } });
    fireEvent.click(screen.getByRole("switch", { name: "Draft replies to new email" }));
    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST");
      expect(post?.url).toContain("/v1/email-bot");
      expect(post?.body).toEqual({ email_allowed: "ada@example.com", email_inbound: true });
    });
    expect((await screen.findByRole("status")).textContent).toContain("will get a drafted reply");
    expect(await screen.findByText("0 of 40 drafts today.")).toBeTruthy();
  });

  it("says why it is paused instead of showing a switch that does nothing", async () => {
    stubFetch({
      ...BASE,
      inbound: true,
      allowed: "ada@example.com",
      allowed_count: 1,
      problem: "the Email Assistant is set to auto — it has to stay read-only to answer email, because an email can say anything",
    });
    render(<EmailBotSection />);
    expect((await screen.findByRole("alert")).textContent).toContain("Paused: the Email Assistant is set to auto");
  });

  it("an empty list is called out: on, but answering nobody", async () => {
    stubFetch({ ...BASE, inbound: true });
    render(<EmailBotSection />);
    expect((await screen.findByRole("alert")).textContent).toContain("no one is listed");
  });
});
