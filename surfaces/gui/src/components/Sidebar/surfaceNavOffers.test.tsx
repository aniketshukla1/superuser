import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { Sidebar } from "../Sidebar";
import type { SessionInfo } from "../../types";
import { NAV } from "../../nav";
import { BENCH_SESSIONS, MIXED, PERSONAS, SESSIONS, baseProps, renderSidebar, stubFetch } from "./Sidebar.helpers";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
describe("Surface nav", () => {

  // "No folder" is a bucket, not a place. There is nothing to open a session in.
  it("offers no + on the folderless group", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    const orphanOnly: SessionInfo[] = [
      ...SESSIONS,
      { session_id: "s-bench-2", title: "unbound work", workspace: "", agent: "bench", model: "m", mode: "interactive", updated_at: "2026-06-30", messages: 1 },
    ];
    render(<Sidebar {...baseProps} sessions={orphanOnly} agent="bench" />);
    await screen.findByTestId("surface-tab-code");
    fireEvent.click(screen.getByTestId("surface-tab-code"));

    const projects = screen.getByTestId("project-first");
    expect(within(projects).getByText("No folder")).toBeTruthy();
    expect(screen.queryByTestId("project-new")).toBeNull();
  });

  // The same failure the New-session button had: the sidebar showing one thing
  // while the chat area shows another.
  it("follows the active session into its own half", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} sessions={BENCH_SESSIONS} agent="bench" />);

    await waitFor(() =>
      expect(screen.getByTestId("surface-tab-code").getAttribute("aria-selected")).toBe("true"),
    );
    expect(screen.getByTestId("project-first")).toBeTruthy();
  });

  // A tab leading to an empty list is worse than no tab. Someone who has never made
  // a workbench should not be offered the distinction at all.
  it("is not offered when there is only one kind", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: PERSONAS },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} />);
    await screen.findByLabelText("Group and filter conversations");

    expect(screen.queryByTestId("surface-tab-chat")).toBeNull();
    expect(screen.queryByTestId("surface-tab-code")).toBeNull();
    // Every persona is still listed — hiding the control must not hide the list.
    expect(screen.getByText("Ops")).toBeTruthy();
  });

  it("keeps the account menu the complete index", async () => {
    await renderSidebar();

    fireEvent.click(screen.getByTestId("account-row"));
    const menu = await screen.findByTestId("account-menu");
    // Every surface, wherever it sits in the rail — nothing becomes unreachable.
    for (const entry of NAV) {
      expect(within(menu).getByText(entry.label)).toBeTruthy();
    }
  });

  // The arrows are a second way to the same two places; with the labels gone, which
  // one is dead IS the "you are here", so it has to track the tab.
  it("moves between Home and Workspace with the arrows, the dead one marking where you are", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} sessions={BENCH_SESSIONS} />);
    await screen.findByTestId("surface-tab-chat");
    const back = () => screen.getByTestId("surface-back") as HTMLButtonElement;
    const forward = () => screen.getByTestId("surface-forward") as HTMLButtonElement;
    expect([back().disabled, forward().disabled]).toEqual([true, false]);

    fireEvent.click(forward());
    expect(screen.getByTestId("project-first")).toBeTruthy();
    expect([back().disabled, forward().disabled]).toEqual([false, true]);

    fireEvent.click(back());
    expect(screen.queryByTestId("project-first")).toBeNull();
    expect(back().disabled).toBe(true);
  });
});
