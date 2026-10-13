import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { Sidebar } from "../Sidebar";
import { RAIL_PRIMARY, RAIL_SECONDARY } from "../../nav";
import { BENCH_SESSIONS, MIXED, PERSONAS, SESSIONS, baseProps, renderSidebar, stubFetch } from "./Sidebar.helpers";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
describe("Surface nav", () => {

  it("shows the primary surfaces, and Brain reaches the Brain surface", async () => {
    await renderSidebar();

    for (const entry of RAIL_PRIMARY) {
      expect(screen.getByTestId(entry.testid).textContent).toContain(entry.label);
    }

    fireEvent.click(screen.getByTestId("nav-brain"));
    expect(baseProps.onOpenBrain).toHaveBeenCalled();
  });

  // More is a disclosure, not a deletion. The distinction only holds if the rows
  // it hides are one click away and still work when they get there.
  it("More reveals the secondary surfaces, and they still navigate", async () => {
    await renderSidebar();

    for (const entry of RAIL_SECONDARY) {
      expect(screen.queryByTestId(entry.testid)).toBeNull();
    }

    fireEvent.click(screen.getByTestId("nav-more"));

    for (const entry of RAIL_SECONDARY) {
      expect(screen.getByTestId(entry.testid).textContent).toContain(entry.label);
    }
    fireEvent.click(screen.getByTestId("nav-automations"));
    expect(baseProps.onOpenScheduled).toHaveBeenCalled();
  });

  // Landing on a secondary surface from a link or a restored session must not
  // leave the current location hidden inside a collapsed row, highlighting nothing.
  // More used to force itself open when you landed on a surface inside it, so the current
  // location was never invisible. It is a popover now, and a popover that opens itself
  // covers the work you just navigated to — so the More ROW carries the active state
  // instead, which says the same thing without seizing the screen.
  it("marks More as current when the surface you are on is hidden inside it", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: PERSONAS },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
    ]);
    render(<Sidebar {...baseProps} auditActive />);
    await screen.findByLabelText("Group and filter conversations");

    const more = screen.getByTestId("nav-more");
    expect(more.getAttribute("aria-expanded")).toBe("false");
    expect(more.className).toContain("text-ink");
    // And the surface is still reachable — hidden, not gone.
    fireEvent.click(more);
    expect(within(screen.getByTestId("more-menu")).getByTestId("nav-activity")).toBeTruthy();
  });

  // A menu the keyboard can open has to be one the keyboard can leave: Escape closes More and
  // puts focus back on the row that opened it, not on the page body.
  it("closes More on Escape and returns focus to it", async () => {
    await renderSidebar();
    const more = screen.getByTestId("nav-more");
    fireEvent.click(more);
    const menu = screen.getByTestId("more-menu");
    expect(menu.contains(document.activeElement)).toBe(true);
    fireEvent.keyDown(menu, { key: "Escape" });
    expect(screen.queryByTestId("more-menu")).toBeNull();
    expect(document.activeElement).toBe(more);
  });

  // Settings was reachable only through the account menu — a menu labelled "Not signed in".
  it("reaches Settings in one click from the rail", async () => {
    await renderSidebar();
    fireEvent.click(screen.getByTestId("settings-button"));
    expect(baseProps.onManage).toHaveBeenCalled();
    expect(screen.queryByTestId("account-menu")).toBeNull();
  });

  // Edit sidebar has to be findable without first hiding something, so More is offered
  // even when it holds nothing else.
  it("offers Edit sidebar from More, and applies a change live", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: PERSONAS },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
    ]);
    render(<Sidebar {...baseProps} />);
    await screen.findByLabelText("Group and filter conversations");

    // Brain ships visible; Privacy ships behind More.
    expect(screen.getByTestId("nav-brain")).toBeTruthy();
    expect(screen.queryByTestId("nav-activity")).toBeNull();

    fireEvent.click(screen.getByTestId("nav-more"));
    fireEvent.click(screen.getByTestId("more-edit-sidebar"));
    const dialog = screen.getByTestId("edit-sidebar");

    // Checking Privacy puts it on the rail without dismissing the dialog: the sidebar is
    // visible behind it, so a checkbox that waits for Done makes you close the thing you
    // are looking at to find out what you did.
    fireEvent.click(within(dialog).getByTestId("edit-sidebar-audit"));
    expect(screen.getByTestId("nav-activity")).toBeTruthy();

    // And unchecking one takes it off.
    fireEvent.click(within(dialog).getByTestId("edit-sidebar-brain"));
    expect(screen.queryByTestId("nav-brain")).toBeNull();
  });

  // A count on a hidden row would otherwise vanish with the row — the one thing a
  // disclosure must never do is hide that something is waiting for you.
  it("never hides a waiting count behind More", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: PERSONAS },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
    ]);
    const withAttention = SESSIONS.map((s) => ({ ...s, attention: 3 }));
    render(<Sidebar {...baseProps} sessions={withAttention} />);
    await screen.findByLabelText("Group and filter conversations");

    // Inbox is primary, so its own badge carries the count on screen. If it is ever
    // moved behind More, the More row has to carry it instead — one of these two is
    // always true, and this asserts the count is visible either way.
    const inboxIsPrimary = RAIL_PRIMARY.some((e) => e.id === "inbox");
    const carrier = inboxIsPrimary
      ? screen.getByTestId("nav-inbox")
      : screen.getByTestId("nav-more");
    expect(carrier.textContent).toContain("6");
  });

  // The switch says which half of the list you are looking at — as icons now, so the name
  // lives in the accessible name and the tooltip. (The arrows beside it say it a second
  // time; surfaceNavOffers.test.tsx holds them to that.)
  it("names each half of the list, and switches between them", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} sessions={BENCH_SESSIONS} />);
    await screen.findByTestId("surface-tab-chat");

    for (const [id, name] of [["chat", "Home"], ["code", "Workspace"]]) {
      const tab = screen.getByTestId(`surface-tab-${id}`);
      expect(tab.getAttribute("aria-label")).toBe(name);
      expect(tab.getAttribute("data-tip")).toBe(name);
      expect(tab.textContent, "icon only — the name is in the tooltip").toBe("");
    }

    fireEvent.click(screen.getByTestId("surface-tab-code"));
    expect(screen.getByTestId("project-first")).toBeTruthy();
    expect(screen.getByTestId("surface-tab-code").getAttribute("aria-selected")).toBe("true");

    fireEvent.click(screen.getByTestId("surface-tab-chat"));
    expect(screen.queryByTestId("project-first")).toBeNull();
  });

  it("Chat lists personas; Code lists projects", async () => {
    stubFetch([
      { match: "/v1/personas", method: "GET", json: MIXED },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} sessions={BENCH_SESSIONS} />);

    // Chat: the persona accordion, and no project list at all.
    await screen.findByTestId("surface-tab-chat");
    expect(screen.queryByTestId("project-first")).toBeNull();
    expect(screen.getByText("Ops")).toBeTruthy();

    fireEvent.click(screen.getByTestId("surface-tab-code"));

    // Code: folders at the top level. The folder name is what you see; the persona
    // that owns the session is not a heading any more.
    const projects = screen.getByTestId("project-first");
    expect(within(projects).getByText("aegis")).toBeTruthy();
    expect(screen.queryByText("Bench")).toBeNull();
  });
});
