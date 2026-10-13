import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { BoardTab } from "./BoardTab";
import { panelTabs } from "./WorkspacePanel";

// What the board view has to get right is which item needs a person. Everything
// else is a list.

const ITEM = {
  id: "i1",
  title: "Add the retry",
  detail: "",
  acceptance: "the flaky test passes ten times",
  state: "review" as const,
  assignee: "abc-sub1",
  builder: "abc-sub1",
  notes: [],
  reviews: [],
};

function stub(items: any[]) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => ({ ok: true, json: async () => ({ items }) }) as Response),
  );
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("BoardTab", () => {
  it("says what an item is waiting for when nobody has checked it", async () => {
    stub([ITEM]);
    render(<BoardTab workspace="/repo" />);
    await screen.findByTestId("board");
    expect(screen.getByText(/needs a reviewer who isn’t the builder/)).toBeTruthy();
  });

  it("leads with what needs a person, not with what is finished", async () => {
    stub([
      { ...ITEM, id: "done1", title: "Finished thing", state: "done" },
      { ...ITEM, id: "rev1", title: "Needs checking" },
    ]);
    render(<BoardTab workspace="/repo" />);
    const board = await screen.findByTestId("board");
    const text = board.textContent || "";
    expect(text.indexOf("Needs checking")).toBeLessThan(text.indexOf("Finished thing"));
  });

  it("shows a failed review as failed, with its evidence", async () => {
    stub([{ ...ITEM, reviews: [{ by: "abc-sub2", pass: false, evidence: "still flaky on run three" }] }]);
    render(<BoardTab workspace="/repo" />);
    fireEvent.click(await screen.findByText("Add the retry"));
    expect(screen.getByText(/Failed by abc-sub2/)).toBeTruthy();
    expect(screen.getByText(/still flaky on run three/)).toBeTruthy();
  });

  it("says out loud when an item has no acceptance criteria", async () => {
    // An item nobody wrote criteria for can only be reviewed on opinion.
    stub([{ ...ITEM, acceptance: "" }]);
    render(<BoardTab workspace="/repo" />);
    fireEvent.click(await screen.findByText("Add the retry"));
    expect(screen.getByText(/No acceptance criteria/)).toBeTruthy();
  });

  it("explains an empty board rather than showing nothing", async () => {
    stub([]);
    render(<BoardTab workspace="/repo" />);
    expect(await screen.findByTestId("board-empty")).toBeTruthy();
  });
});

describe("panelTabs", () => {
  it("hides the Board tab until the project has one", () => {
    // Same rule Terminal and Browser follow: a tab that stands empty for most
    // conversations crowds the strip you scan to find what a turn produced.
    expect(panelTabs([], false).some((t) => t.id === "board")).toBe(false);
    expect(panelTabs([], true).some((t) => t.id === "board")).toBe(true);
  });
});
