import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { RiskOverridesCard } from "./SettingsView";

// The card's real job is not the list — it is the `effective` flag. A rule that
// asks to loosen a built-in is ignored by design, and a rule that vanishes
// silently is how someone concludes the feature is broken.

function stubFetch(rules: any[], saveOk = true) {
  const calls: { url: string; body: any; method?: string }[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    calls.push({ url: String(url), method: init?.method, body: init?.body ? JSON.parse(String(init.body)) : null });
    if (init?.method === "POST") {
      const sent = JSON.parse(String(init.body)).rules;
      return {
        ok: true,
        json: async () =>
          saveOk
            ? { ok: true, rules: sent.map((r: any) => ({ ...r, effective: r.pattern !== "run_shell" })) }
            : { ok: false, error: "nope" },
      } as Response;
    }
    return { ok: true, json: async () => ({ rules }) } as Response;
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("RiskOverridesCard", () => {
  it("shows an ignored rule rather than hiding it", async () => {
    stubFetch([
      { pattern: "notion_*", risk: "read", effective: true },
      { pattern: "run_shell", risk: "read", effective: false },
    ]);
    render(<RiskOverridesCard />);
    await screen.findByTestId("risk-rules");
    expect(screen.getByText("run_shell")).toBeTruthy();
    expect(screen.getByText(/can’t loosen a built-in/)).toBeTruthy();
    // The effective one carries no such note.
    expect(screen.getAllByText(/can’t loosen a built-in/).length).toBe(1);
  });

  it("adds a rule on top of the ones already there", async () => {
    const calls = stubFetch([{ pattern: "notion_*", risk: "read", effective: true }]);
    render(<RiskOverridesCard />);
    await screen.findByTestId("risk-rules");

    fireEvent.change(screen.getByTestId("risk-pattern"), { target: { value: "linear_delete_*" } });
    fireEvent.change(screen.getByTestId("risk-class"), { target: { value: "external" } });
    fireEvent.click(screen.getByText("Add"));

    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST");
      expect(post).toBeTruthy();
      // The existing rule must survive — this is a whole-list write.
      expect(post!.body.rules).toEqual([
        { pattern: "notion_*", risk: "read" },
        { pattern: "linear_delete_*", risk: "external" },
      ]);
    });
  });

  it("removes only the rule asked for", async () => {
    const calls = stubFetch([
      { pattern: "a_*", risk: "read", effective: true },
      { pattern: "b_*", risk: "external", effective: true },
    ]);
    render(<RiskOverridesCard />);
    await screen.findByTestId("risk-rules");
    fireEvent.click(screen.getAllByText("Remove")[0]);

    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST");
      expect(post!.body.rules).toEqual([{ pattern: "b_*", risk: "external" }]);
    });
  });

  it("says nothing is overridden rather than looking broken when empty", async () => {
    stubFetch([]);
    render(<RiskOverridesCard />);
    await screen.findByTestId("risk-overrides-card");
    expect(screen.queryByTestId("risk-rules")).toBeNull();
    expect(screen.getByTestId("risk-pattern")).toBeTruthy();
  });
});
