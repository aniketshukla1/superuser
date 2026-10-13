import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { Sidebar } from "../Sidebar";
import type { SessionInfo } from "../../types";
import { BENCH_SESSIONS, MIXED, SESSIONS, baseProps, codeTab, openFolders, stubFetch } from "./Sidebar.helpers";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
describe("Surface nav", () => {

  // A workbench session with no folder is still work. Dropping it because it has
  // nothing to group under would lose it from the sidebar entirely.
  it("keeps folderless workbench sessions instead of dropping them", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    const orphan: SessionInfo[] = [
      ...SESSIONS,
      { session_id: "s-bench-2", title: "unbound work", workspace: "", agent: "bench", model: "m", mode: "interactive", updated_at: "2026-06-30", messages: 1 },
    ];
    render(<Sidebar {...baseProps} sessions={orphan} />);
    await screen.findByTestId("surface-tab-code");
    fireEvent.click(screen.getByTestId("surface-tab-code"));

    const projects = screen.getByTestId("project-first");
    expect(within(projects).getByText("No folder")).toBeTruthy();
    expect(within(projects).getByText("unbound work")).toBeTruthy();
  });

  it("opens folders one at a time, like the persona headers", async () => {
    await codeTab({ workspace: "/w/aegis" });
    expect(openFolders()).toEqual(["aegis"]); // the one you are in

    fireEvent.click(screen.getByText("borealis"));
    expect(openFolders()).toEqual(["borealis"]); // and aegis closed itself

    fireEvent.click(screen.getByText("cirrus"));
    expect(openFolders()).toEqual(["cirrus"]);
  });

  it("closes the open folder when you click it again", async () => {
    await codeTab({ workspace: "/w/aegis" });
    fireEvent.click(screen.getByText("aegis"));
    expect(openFolders()).toEqual([]);
  });

  it("opens the top folder when no workspace is bound, so the list is never shut", async () => {
    await codeTab({ workspace: "" });
    expect(openFolders().length).toBe(1);
  });

  // A project header is a label, not a control. It carried a folder icon, a rotating
  // chevron and a session count; the artifact and the reference both have just the name
  // and a +. The count in particular restated what the rows below already showed.
  it("shows a project as a name and a +, not a control panel", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} sessions={BENCH_SESSIONS} agent="bench" />);
    await screen.findByTestId("surface-tab-code");
    fireEvent.click(screen.getByTestId("surface-tab-code"));

    const projects = screen.getByTestId("project-first");
    const header = within(projects).getByText("aegis").parentElement!;
    // The + and the collapse chevron stay; the folder icon and the session count — which
    // restated what the rows below already showed — do not.
    expect(within(header).getByTestId("project-new")).toBeTruthy();
    expect(header.querySelectorAll("svg").length).toBe(2);
    expect(header.textContent).toBe("aegis");
  });

  // Workspace folders collapse the way Home's persona groups do. Removing this on the
  // grounds that `peek` caps each group was wrong: capping a list is not the same as
  // being able to put one away.
  it("collapses a project without collapsing the + inside its header", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    const onNewInProject = vi.fn();
    render(
      <Sidebar {...baseProps} sessions={BENCH_SESSIONS} agent="bench" onNewInProject={onNewInProject} />,
    );
    await screen.findByTestId("surface-tab-code");
    fireEvent.click(screen.getByTestId("surface-tab-code"));

    const header = within(screen.getByTestId("project-first")).getByText("aegis").parentElement!;
    expect(within(screen.getByTestId("project-first")).getByText("refactor the parser")).toBeTruthy();

    fireEvent.click(header);
    expect(
      within(screen.getByTestId("project-first")).queryByText("refactor the parser"),
    ).toBeNull();

    fireEvent.click(header);
    expect(within(screen.getByTestId("project-first")).getByText("refactor the parser")).toBeTruthy();

    // The + must not also toggle the row it sits in.
    fireEvent.click(within(header).getByTestId("project-new"));
    expect(onNewInProject).toHaveBeenCalled();
    expect(within(screen.getByTestId("project-first")).getByText("refactor the parser")).toBeTruthy();
  });

  // "Recent" labels a chronological list, which is what Chat shows. Under Code the list is
  // projects, so the label described something that was not there.
  it("drops the Recent header under Code and keeps it under Chat", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} sessions={BENCH_SESSIONS} />);
    await screen.findByTestId("surface-tab-chat");
    expect(screen.getByTestId("recent-header")).toBeTruthy();

    fireEvent.click(screen.getByTestId("surface-tab-code"));
    expect(screen.queryByTestId("recent-header")).toBeNull();
  });

  // The per-project `+`. The distinction that matters: onNewProject opens the folder
  // gate because no folder is known; this one already knows, so asking again would be
  // a question we can answer ourselves.
  it("starts a session in the folder without asking which folder", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    const onNewInProject = vi.fn();
    const onNewProject = vi.fn();
    render(
      <Sidebar
        {...baseProps}
        sessions={BENCH_SESSIONS}
        agent="bench"
        onNewInProject={onNewInProject}
        onNewProject={onNewProject}
      />,
    );
    await screen.findByTestId("surface-tab-code");
    fireEvent.click(screen.getByTestId("surface-tab-code"));

    fireEvent.click(screen.getByTestId("project-new"));
    expect(onNewInProject).toHaveBeenCalledWith("/w/aegis", "bench");
    // Not the gate. Being sent to "which folder?" when you clicked + ON a folder is
    // the whole bug this button exists to avoid.
    expect(onNewProject).not.toHaveBeenCalled();
  });

  // The + sits inside the row that toggles the folder. Without stopPropagation,
  // starting a session also shuts the list you started it from.
  it("does not collapse the folder it starts a session in", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} sessions={BENCH_SESSIONS} agent="bench" />);
    await screen.findByTestId("surface-tab-code");
    fireEvent.click(screen.getByTestId("surface-tab-code"));

    const projects = screen.getByTestId("project-first");
    expect(within(projects).getByText("refactor the parser")).toBeTruthy();
    fireEvent.click(screen.getByTestId("project-new"));
    expect(within(screen.getByTestId("project-first")).getByText("refactor the parser")).toBeTruthy();
  });
});
