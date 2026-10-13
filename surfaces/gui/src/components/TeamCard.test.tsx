import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { TeamCard } from "./SettingsView";

// The card exists because the extension point was invisible. What it has to convey
// is which types exist, which are yours, and who staffs them.

const DATA = {
  types: [
    { name: "general-purpose", description: "Multi-step work", builtin: true, tools: null, instructions: "" },
    { name: "verify", description: "Check someone else's work", builtin: true, tools: ["read_file", "run_shell"], instructions: "" },
    { name: "my-worker", description: "Mine", builtin: false, tools: ["read_file"], instructions: "" },
  ],
  rosters: [{ id: "lead", name: "Team Lead", subagents: ["general-purpose", "verify"] }],
};

function stub(data: any = DATA) {
  vi.stubGlobal("fetch", vi.fn(async () => ({ ok: true, json: async () => data }) as Response));
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("TeamCard", () => {
  it("says who staffs each worker type", async () => {
    stub();
    render(<TeamCard />);
    await screen.findByTestId("worker-types");
    const rows = screen.getByTestId("worker-types").textContent || "";
    expect(rows).toContain("staffed by Team Lead");
  });

  it("distinguishes a fixed toolset from an inherited one", async () => {
    // "2 tools, fixed" and "whatever the assistant can use" are different promises
    // about what a delegate may do, and the difference is the security property.
    stub();
    render(<TeamCard />);
    const rows = await screen.findByTestId("worker-types");
    expect(rows.textContent).toContain("Uses whatever the assistant that spawned it can use");
    expect(rows.textContent).toContain("2 tools, fixed");
  });

  it("marks the types you wrote yourself", async () => {
    stub();
    render(<TeamCard />);
    await screen.findByTestId("worker-types");
    expect(screen.getByText("yours")).toBeTruthy();
    expect(screen.getAllByText("yours").length).toBe(1);
  });

  it("points at the folder rather than leaving it undiscoverable", async () => {
    stub({ types: [], rosters: [] });
    render(<TeamCard />);
    const card = await screen.findByTestId("team-card");
    expect(card.textContent).toContain("subagents/");
  });
});
