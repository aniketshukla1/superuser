import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { Sidebar } from "../Sidebar";
import { PERSONAS, baseProps, stubFetch } from "./Sidebar.helpers";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
describe("the Scheduled band", () => {
  // The endpoint answers under "tasks" (see api.getAutomations), not "automations".
  const AUTOMATIONS = {
    tasks: [
      { id: "a1", title: "Morning brief", schedule: "Every day at ~7:00 AM", enabled: true },
      { id: "a2", title: "Weekly digest", schedule: "Mondays at 9:00 AM", enabled: true },
    ],
  };
  const withAutomations = () =>
    stubFetch([
      { match: "/v1/personas", json: PERSONAS },
      { match: "/v1/automations", json: AUTOMATIONS },
    ]);

  it("adds a new automation from the band, without opening one of the existing ones", async () => {
    // The one band you could not add to from the sidebar was the one whose whole
    // point is work that happens without you.
    withAutomations();
    const onNewAutomation = vi.fn();
    const onOpenAutomation = vi.fn();
    render(<Sidebar {...baseProps} onNewAutomation={onNewAutomation} onOpenAutomation={onOpenAutomation} />);
    const plus = await waitFor(() => screen.getByTestId("scheduled-new"));
    fireEvent.click(plus);
    expect(onNewAutomation).toHaveBeenCalledTimes(1);
    // The "+" sits inside a row that folds the band; clicking it must not do both.
    expect(onOpenAutomation).not.toHaveBeenCalled();
    expect(screen.getByTestId("scheduled-a1")).toBeTruthy();
  });

  it("folds and unfolds the list from its header", async () => {
    withAutomations();
    render(<Sidebar {...baseProps} />);
    const header = await waitFor(() => screen.getByTestId("scheduled-header"));
    const list = () => screen.getByTestId("scheduled-list") as HTMLElement;
    expect(header.getAttribute("aria-expanded")).toBe("true"); // open by default
    expect(list().hidden).toBe(false);

    fireEvent.click(header);
    expect(header.getAttribute("aria-expanded")).toBe("false");
    expect(list().hidden).toBe(true);

    fireEvent.click(header);
    expect(header.getAttribute("aria-expanded")).toBe("true");
    expect(list().hidden).toBe(false);
  });
});

// "In picker" is one switch, and it has to mean one thing all the way through.
// It didn't: the new-session menu filtered on `enabled` alone, so an assistant you
// had just hidden was still offered there — and picking it opened a conversation
// the sidebar had no section for, because a session with no turns yet is in no
// session list. Hidden in one place, offered in another, and the conversation
// nowhere at all.
describe("hiding an assistant from the picker", () => {
  const HIDDEN = {
    personas: [
      { id: "cowork", name: "Superuser", icon: "cowork", tagline: "general", family: "knowledge", enabled: true, surfaced: true, default: true },
      { id: "ops", name: "Ops", icon: "ops", tagline: "runbooks", family: "code", enabled: true, surfaced: true, default: false },
      // Enabled, but the user turned "In picker" off.
      { id: "support", name: "Customer Support", icon: "helpCircle", tagline: "tickets", family: "knowledge", enabled: true, surfaced: false, default: false },
    ],
  };

  it("takes it out of the New menu", async () => {
    stubFetch([{ match: "/v1/personas", json: HIDDEN }]);
    const { container } = render(<Sidebar {...baseProps} />);
    await waitFor(() => expect(screen.getByLabelText("Choose a persona")).toBeTruthy());
    fireEvent.click(screen.getByLabelText("Choose a persona"));
    const menu = await waitFor(() => {
      const m = container.querySelector(".newsplit-menu");
      if (!m) throw new Error("menu did not open");
      return m as HTMLElement;
    });
    // Scoped to the menu: these names also appear in the sidebar's own list.
    expect(within(menu).queryByText("Customer Support")).toBeNull();
    expect(within(menu).getByText("Ops")).toBeTruthy(); // the ones still in it are unaffected
  });

  it("still gives its conversation a home when that is the one you are in", async () => {
    // The other half. However you got here — a deep link, a resumed session — the
    // assistant you are TALKING TO must appear, or the chat area shows one thing
    // and the left nav denies it exists.
    stubFetch([{ match: "/v1/personas", json: HIDDEN }]);
    render(<Sidebar {...baseProps} agent="support" sessions={[]} activeSession="s-new" />);
    await waitFor(() => expect(screen.getAllByText("Customer Support").length).toBeGreaterThan(0));
  });

  it("keeps it out of the sidebar when you are not talking to it", async () => {
    // And the switch still does its job the rest of the time.
    stubFetch([{ match: "/v1/personas", json: HIDDEN }]);
    render(<Sidebar {...baseProps} agent="cowork" sessions={[]} />);
    await waitFor(() => expect(screen.getByText("Ops")).toBeTruthy());
    expect(screen.queryByText("Customer Support")).toBeNull();
  });
});

describe("Sidebar group/filter control", () => {
  it("choosing Persona persists via setNavLayout and switches to the per-persona accordion", async () => {
    const calls = stubFetch([
      { match: "/v1/personas", method: "GET", json: PERSONAS },
      { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
      { match: "/v1/settings/nav-layout", method: "POST", json: { ok: true, nav_layout: "grouped" } },
    ]);
    render(<Sidebar {...baseProps} />);

    // personas load drives the surfaces; the RECENT header's group/filter control is always present.
    const control = await screen.findByLabelText("Group and filter conversations");

    // Open the popover and choose "Group by → Persona".
    fireEvent.click(control);
    fireEvent.click(await screen.findByText("Persona"));

    // POSTs the new layout pref.
    await waitFor(() => {
      const post = calls.find((c) => c.method === "POST" && c.url.includes("/v1/settings/nav-layout"));
      expect(post).toBeTruthy();
      expect(post!.body).toMatchObject({ nav_layout: "grouped" });
    });

    // Close the popover (it stays open so you can group AND filter in one visit) before asserting
    // the accordion — otherwise "Ops" also matches the filter-by-assistant checkbox.
    fireEvent.click(control);

    // Grouped view = the per-persona accordion. The Ops header appears; expanding it lists its
    // session. (Persona configuration moved to Settings ▸ Personas, so there is no header gear.)
    const opsHeader = await screen.findByText("Ops");
    fireEvent.click(opsHeader);
    expect(screen.getByText("incident watch")).toBeTruthy();
    expect(screen.queryByTitle("About the Ops persona")).toBeNull();
  });
});
