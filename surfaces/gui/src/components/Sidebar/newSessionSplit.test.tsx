import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { Sidebar } from "../Sidebar";
import { PERSONAS, baseProps, stubFetch } from "./Sidebar.helpers";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
describe("New-session split button", () => {
  it("collapses to a plain button when only one persona is enabled", async () => {
    stubFetch([
      {
        match: "/v1/personas",
        method: "GET",
        json: { personas: [PERSONAS.personas[0], PERSONAS.personas[3]] }, // cowork + a disabled one
      },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
    ]);
    const { container } = render(<Sidebar {...baseProps} />);
    await screen.findByText("incident watch");

    // No ▾ — nothing to pick; the primary button starts the sole enabled persona.
    await waitFor(() => expect(screen.queryByLabelText("Choose a persona")).toBeNull());
    fireEvent.click(container.querySelector(".newsplit-primary")!);
    expect(baseProps.onNewSession).toHaveBeenCalledWith("cowork");
  });

  it("primary starts the last-used persona; the menu lists enabled personas + Manage personas…", async () => {
    localStorage.setItem("superuser.flag.personas", "1"); // Manage entry is launch-flagged off
    stubFetch([
      { match: "/v1/personas", method: "GET", json: PERSONAS },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
    ]);
    const { container } = render(<Sidebar {...baseProps} />);
    await screen.findByLabelText("Group and filter conversations");

    // Primary action → a new session with the current (last-used) persona.
    fireEvent.click(container.querySelector(".newsplit-primary")!);
    expect(baseProps.onNewSession).toHaveBeenCalledWith("cowork");

    // The row reads "New" — it sits in a column of 13px rows and "New session" would be the
    // only two-word one — but it is ANNOUNCED as "New session", which is also how four e2e
    // specs and the collapsed topbar cluster find it. Dropping the aria-label to "simplify"
    // would leave screen readers hearing a bare "New" and break those specs with an error
    // about a missing button rather than a missing label.
    const primary = container.querySelector(".newsplit-primary")!;
    expect(primary.textContent).toBe("New");
    expect(primary.getAttribute("aria-label")).toBe("New session");
    expect(screen.getByRole("button", { name: "New session" })).toBe(primary);

    // New project used to be a folder-plus button beside a "PROJECTS" heading. Both are
    // gone, so the menu is the only way to start one — the same trade the New-assistant
    // row made, and the same reason to test it.
    fireEvent.click(screen.getByLabelText("Choose a persona"));
    fireEvent.click(await screen.findByTestId("menu-new-project"));
    expect(baseProps.onNewProject).toHaveBeenCalled();

    // Creating an assistant used to have its own sidebar row AND this entry. The row is
    // gone, so this is the only way in — which makes it worth a test, where it had none
    // while the row existed to cover for it.
    fireEvent.click(screen.getByLabelText("Choose a persona"));
    fireEvent.click(await screen.findByText("+ New assistant…"));
    expect(screen.getByRole("dialog", { name: "New assistant" })).toBeTruthy();
    fireEvent.keyDown(document, { key: "Escape" });

    // ▾ opens the persona menu: enabled personas appear, the disabled one does not, plus a manage entry.
    fireEvent.click(screen.getByLabelText("Choose a persona"));
    const menu = (await screen.findByText("Start a session as")).closest(".newsplit-menu") as HTMLElement;
    const w = within(menu);
    expect(w.getByText("Ops")).toBeTruthy();
    expect(w.getByText("Code")).toBeTruthy();
    expect(w.queryByText("Disabled One")).toBeNull();
    expect(w.getByText("Manage personas…")).toBeTruthy();

    // Selecting a persona starts a session as that persona.
    fireEvent.click(w.getByText("Ops"));
    expect(baseProps.onNewSession).toHaveBeenCalledWith("ops");

    // "Manage personas…" opens the persona management surface.
    fireEvent.click(screen.getByLabelText("Choose a persona"));
    fireEvent.click(await screen.findByText("Manage personas…"));
    expect(baseProps.onManagePersonas).toHaveBeenCalled();
  });

  // Reported: with Code active, clicking Assistant and then New session opened a
  // CODE session. Expanding a persona browses it without switching the chat area,
  // and the split button was reading the ACTIVE persona rather than the browsed
  // one — so the click looked ignored, because its only visible effect was in a
  // pane you weren't looking at.
  //
  // It was harmless until the per-persona New-session action was removed from each
  // expanded body; after that this button was the only way to start a session in a
  // persona you were browsing, and it started the wrong one.
  it("starts a session in the persona you are browsing, not the active one", async () => {
    stubFetch([
      { match: "/v1/personas", json: PERSONAS },
      { match: "/v1/settings", json: { nav_layout: "grouped" } },
    ]);
    const onNewSession = vi.fn();
    // "code" is the ACTIVE persona; the user is about to browse a different one.
    const { container } = render(
      <Sidebar {...baseProps} agent="code" onNewSession={onNewSession} />,
    );

    // Browse Ops by expanding its header. This must NOT switch the chat area.
    fireEvent.click(await screen.findByText("Ops"));
    expect(baseProps.onSwitchAgent).not.toHaveBeenCalled();

    // Now the primary action starts Ops — the persona on screen — not "code".
    fireEvent.click(container.querySelector(".newsplit-primary")!);
    expect(onNewSession).toHaveBeenCalledWith("ops");
  });

  // The fallback half of the same rule: with nothing expanded, the button still
  // starts the active persona. Fixing the case above must not break this one.
  it("falls back to the active persona when no accordion is open", async () => {
    stubFetch([
      { match: "/v1/personas", json: PERSONAS },
      { match: "/v1/settings", json: { nav_layout: "grouped" } },
    ]);
    const onNewSession = vi.fn();
    const { container } = render(
      <Sidebar {...baseProps} agent="ops" onNewSession={onNewSession} />,
    );
    await screen.findByLabelText("Group and filter conversations");

    // Collapse the active persona's own accordion, so nothing is being browsed.
    fireEvent.click(await screen.findByText("Ops"));
    fireEvent.click(container.querySelector(".newsplit-primary")!);
    expect(onNewSession).toHaveBeenCalledWith("ops");
  });

  it("shows Manage personas… by default, and hides it when the flag is explicitly off", async () => {
    // Superuser ships personas ON (they're the organising idea); "0" force-hides.
    localStorage.setItem("superuser.flag.personas", "0");
    stubFetch([
      { match: "/v1/personas", method: "GET", json: PERSONAS },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
    ]);
    render(<Sidebar {...baseProps} />);
    await screen.findByLabelText("Group and filter conversations");
    fireEvent.click(screen.getByLabelText("Choose a persona"));
    const menu = (await screen.findByText("Start a session as")).closest(".newsplit-menu") as HTMLElement;
    expect(within(menu).getByText("Ops")).toBeTruthy();
    expect(within(menu).queryByText("Manage personas…")).toBeNull();
    localStorage.removeItem("superuser.flag.personas");
  });
});
