import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ApprovalsCard } from "./SettingsView";

// The two switches that decide how much happens without the user looking. What
// matters here is not that a checkbox toggles — it is that the checkbox never
// claims a setting the server did not actually store, and that turning one on
// cannot turn the other off behind the user's back.

function stubFetch(settings: Record<string, unknown>, saveOk = true) {
  const calls: { url: string; body: any }[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : null });
    if (String(url).includes("/v1/settings/approvals")) {
      return {
        ok: true,
        json: async () => (saveOk ? { ok: true, ...JSON.parse(String(init?.body ?? "{}")) } : { ok: false, error: "nope" }),
      } as Response;
    }
    return { ok: true, json: async () => settings } as Response;
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("ApprovalsCard", () => {
  it("shows both switches off until the server says otherwise", async () => {
    stubFetch({});
    render(<ApprovalsCard />);
    const readOnly = await screen.findByTestId("readonly-commands-toggle");
    const auto = screen.getByTestId("auto-approve-toggle");
    expect(readOnly.getAttribute("aria-checked")).toBe("false");
    expect(auto.getAttribute("aria-checked")).toBe("false");
  });

  it("reflects what the server already has on", async () => {
    stubFetch({ auto_approve: true, auto_allow_read_only_commands: true });
    render(<ApprovalsCard />);
    const auto = await screen.findByTestId("auto-approve-toggle");
    expect(auto.getAttribute("aria-checked")).toBe("true");
    expect(screen.getByTestId("readonly-commands-toggle").getAttribute("aria-checked")).toBe("true");
  });

  it("sends only the switch that changed, so one cannot silently clear the other", async () => {
    const calls = stubFetch({ auto_approve: true });
    render(<ApprovalsCard />);
    const readOnly = await screen.findByTestId("readonly-commands-toggle");
    fireEvent.click(readOnly);

    await waitFor(() => {
      const save = calls.find((c) => c.url.includes("/v1/settings/approvals"));
      expect(save).toBeTruthy();
      expect(save!.body).toEqual({ auto_allow_read_only_commands: true });
    });
  });

  it("puts the switch back when the save fails", async () => {
    // Otherwise the panel says a setting is on that the server never stored, and
    // the user believes commands are being auto-approved when they are not — or,
    // worse, believes they are not when they are.
    stubFetch({}, false);
    render(<ApprovalsCard />);
    const auto = await screen.findByTestId("auto-approve-toggle");
    fireEvent.click(auto);
    await waitFor(() => expect(auto.getAttribute("aria-checked")).toBe("false"));
  });
});
