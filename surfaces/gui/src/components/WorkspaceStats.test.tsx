import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { statsOf, WorkspaceStats } from "./WorkspaceStats";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("statsOf", () => {
  // The panel is the first thing a new chat renders; a throw here unloads the whole main area.
  it("fills in every field a payload leaves out", () => {
    const s = statsOf({ ok: true })!;
    expect(s.sessions).toBe(0);
    expect(s.models).toEqual([]);
    expect(s.peak_hour).toBeNull();
  });

  it("is null for nothing at all", () => {
    expect(statsOf(null)).toBeNull();
    expect(statsOf("nope")).toBeNull();
  });
});

describe("WorkspaceStats", () => {
  function stub(payload: unknown) {
    vi.stubGlobal("fetch", vi.fn(async () => ({ ok: true, json: async () => payload }) as Response));
  }

  it("renders a partial payload instead of throwing", async () => {
    stub({ sessions: 3, messages: 12 });
    render(<WorkspaceStats />);
    expect(await screen.findByText("Sessions")).toBeTruthy();
  });

  it("does not report a streak of zero, and names the model without its routing prefix", async () => {
    stub({
      sessions: 62,
      messages: 234,
      current_streak: 0,
      longest_streak: 2,
      favorite_model: "custom:qwen2.5:7b-instruct",
      favorite_agent: "assistant",
    });
    render(<WorkspaceStats />);
    await screen.findByText("Sessions");
    expect(screen.queryByText("Current streak")).toBeNull();
    expect(screen.getByText("2 days")).toBeTruthy();
    expect(screen.getByText("qwen2.5:7b-instruct")).toBeTruthy();
    expect(screen.getByText("Assistant")).toBeTruthy();
  });
});
