import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SkillsTab } from "../SkillsTab";
import { LIST, PREVIEW, openLinkDoor, served, stubFetch } from "./SkillsTab.helpers";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
describe("SkillsTab", () => {
  it("renders rows with provenance badges and dims disabled skills", async () => {
    stubFetch([{ match: "/v1/skills", method: "GET", json: LIST }]);
    render(<SkillsTab />);
    expect(await screen.findByText("weekly-report")).toBeTruthy();
    expect(screen.getByText("Monday status report")).toBeTruthy();
    expect(screen.queryByText("global")).toBeNull(); // no scope badges — global-only (§4.7)
    expect(screen.getByText("uploaded")).toBeTruthy(); // provenance badge stays
    const toggles = screen.getAllByRole("switch");
    expect((toggles[0] as HTMLInputElement).checked).toBe(true);
    expect((toggles[1] as HTMLInputElement).checked).toBe(false);
  });


  it("delete is two-step: arm, then DELETE on confirm", async () => {
    const calls = stubFetch([
      { match: "/v1/skills", method: "GET", json: LIST },
      { match: "/v1/skills/weekly-report", method: "DELETE", json: { ok: true } },
    ]);
    render(<SkillsTab />);
    await screen.findByText("weekly-report");
    // arm via the trash button (renders "Confirm delete" once armed)
    fireEvent.click(screen.getByLabelText("Delete weekly-report"));
    expect(calls.some((c) => c.method === "DELETE")).toBe(false);
    const confirm = await screen.findByText("Confirm delete");
    fireEvent.click(confirm);
    await waitFor(() => {
      expect(calls.some((c) => c.method === "DELETE" && c.url.includes("weekly-report"))).toBe(true);
    });
  });


  it("the enabled switch PATCHes {enabled} and teaches the off rule + physics footnote", async () => {
    const calls = stubFetch([
      { match: "/v1/skills", method: "GET", json: LIST },
      { match: "/v1/skills/weekly-report", method: "PATCH", json: { ok: true } },
    ]);
    render(<SkillsTab />);
    await screen.findByText("weekly-report");
    fireEvent.click(screen.getByLabelText("weekly-report enabled"));
    await waitFor(() => {
      const patch = calls.find((c) => c.method === "PATCH");
      expect(patch?.body).toMatchObject({ enabled: false });
    });
    const status = await screen.findByRole("status");
    expect(status.textContent).toContain("weekly-report"); // name-first — WHICH skill
    expect(status.textContent).toContain("turned off everywhere");
    expect(status.textContent).toContain("clean slate"); // the guaranteed remedy, in place
  });


  it("Add skill menu: two doors that work; Create with Superuser hands off to a conversation", async () => {
    const calls = stubFetch([{ match: "/v1/skills", method: "GET", json: { skills: [] } }]);
    const onCreateSkill = vi.fn();
    render(<SkillsTab onCreateSkill={onCreateSkill} />);
    fireEvent.click(await screen.findByRole("button", { name: /Add skill/ }));
    expect(screen.getAllByRole("menuitem")).toHaveLength(2);
    expect(screen.getByText("Install from a link")).toBeTruthy();
    expect(screen.getByText(/asks before adding it to\s+your skills/)).toBeTruthy();
    // Nothing on the server writes a skill from a form or an uploaded file (F-9).
    expect(screen.queryByText("Write it myself")).toBeNull();
    expect(screen.queryByText("Import a file")).toBeNull();
    expect(screen.queryByLabelText("Upload a skill archive")).toBeNull();
    fireEvent.click(screen.getByText("Create with Superuser"));
    // Straight to the conversation — the composer is where you describe it (§5.2).
    expect(onCreateSkill).toHaveBeenCalledWith("");
    // Settings never drafts: no POST of any kind happened.
    expect(calls.some((c) => c.method === "POST")).toBe(false);
  });

  it("a row offers the switch and Delete, and nothing the server doesn't do", async () => {
    stubFetch([{ match: "/v1/skills", method: "GET", json: LIST }]);
    render(<SkillsTab />);
    await screen.findByText("weekly-report");
    expect(screen.getByLabelText("weekly-report enabled")).toBeTruthy();
    expect(screen.getByLabelText("Delete weekly-report")).toBeTruthy();
    expect(screen.queryByTitle("Edit")).toBeNull();
    expect(screen.queryByTitle("Show folder")).toBeNull();
  });

  it("every request the page makes is one the server serves", async () => {
    const calls = stubFetch([
      { match: "/v1/skills/preview", method: "POST", json: { ok: true, preview: PREVIEW } },
      { match: "/v1/skills/install", method: "POST", json: { ok: true, name: "weekly" } },
      { match: "/v1/skills/weekly-report", method: "PATCH", json: { ok: true } },
      { match: "/v1/skills/weekly-report", method: "DELETE", json: { ok: true } },
      { match: "/v1/skills", method: "GET", json: LIST },
    ]);
    render(<SkillsTab onCreateSkill={vi.fn()} />);
    await screen.findByText("weekly-report");
    // Everything the page can do: install from a link, switch, delete, and the menu itself.
    await openLinkDoor();
    fireEvent.change(screen.getByPlaceholderText(/owner\/repo/), { target: { value: "owner/repo" } });
    fireEvent.click(screen.getByText("Fetch"));
    fireEvent.click(await screen.findByText("Install skill"));
    await screen.findByRole("status");
    fireEvent.click(screen.getByLabelText("weekly-report enabled"));
    fireEvent.click(screen.getByLabelText("Delete weekly-report"));
    fireEvent.click(await screen.findByText("Confirm delete"));
    await waitFor(() => expect(calls.some((c) => c.method === "DELETE")).toBe(true));
    fireEvent.click(screen.getByRole("button", { name: /Add skill/ }));
    fireEvent.click(screen.getByText("Create with Superuser"));

    expect(calls.map((c) => c.method).sort()).toEqual(expect.arrayContaining(["DELETE", "GET", "PATCH", "POST"]));
    const unserved = calls.filter((c) => !served(c)).map((c) => `${c.method} ${c.url}`);
    expect(unserved).toEqual([]);
  });

  it("offers no scope UI at all — skills are global (§4.7)", async () => {
    stubFetch([{ match: "/v1/skills", method: "GET", json: { skills: [] } }]);
    render(<SkillsTab />);
    await openLinkDoor();
    expect(screen.queryByText("Available in")).toBeNull();
    expect(screen.queryByLabelText("Everywhere")).toBeNull();
    expect(screen.queryByLabelText("Only one project")).toBeNull();
    expect(screen.queryByText(/Move to/)).toBeNull();
  });

  it("shows the new-session confirmation line after installing a skill", async () => {
    stubFetch([
      { match: "/v1/skills/preview", method: "POST", json: { ok: true, preview: PREVIEW } },
      { match: "/v1/skills/install", method: "POST", json: { ok: true, name: "weekly" } },
      { match: "/v1/skills", method: "GET", json: { skills: [] } },
    ]);
    render(<SkillsTab />);
    await openLinkDoor();
    fireEvent.change(screen.getByPlaceholderText(/owner\/repo/), { target: { value: "owner/repo" } });
    fireEvent.click(screen.getByText("Fetch"));
    fireEvent.click(await screen.findByText("Install skill"));
    const status = await screen.findByRole("status");
    expect(status.textContent).toContain("weekly"); // name-first — WHICH skill
    expect(status.textContent).toContain("can now use it in every conversation");
  });

  it("the list is the page: no standing add-surfaces, no drafting remnants", async () => {
    stubFetch([{ match: "/v1/skills", method: "GET", json: { skills: [] } }]);
    render(<SkillsTab onCreateSkill={vi.fn()} />);
    await screen.findByRole("button", { name: /Add skill/ });
    // No permanently-open description box or draft-era UI (§5.2/§9) — adding is menu-only.
    expect(screen.queryByLabelText("Describe the skill")).toBeNull();
    expect(screen.queryByText("Start a conversation")).toBeNull();
    expect(screen.queryByText("Ask Superuser to revise")).toBeNull();
    expect(screen.queryByText(/Not a chat/)).toBeNull();
    // The menu closes after picking a door.
    await openLinkDoor();
    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByText("Fetch")).toBeTruthy();
  });

  it("surfaces server-side errors", async () => {
    stubFetch([
      { match: "/v1/skills/preview", method: "POST", json: { ok: false, error: "No SKILL.md at that address." } },
      { match: "/v1/skills", method: "GET", json: { skills: [] } },
    ]);
    render(<SkillsTab />);
    await openLinkDoor();
    fireEvent.change(screen.getByPlaceholderText(/owner\/repo/), { target: { value: "owner/nothing" } });
    fireEvent.click(screen.getByText("Fetch"));
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText(/No SKILL.md at that address/)).toBeTruthy();
  });
});
