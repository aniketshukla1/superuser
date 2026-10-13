import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PersonasTab } from "./PersonasTab";

// Importing an agent package: the folder is read into a plan and nothing is written until
// Import; what is refused says why; what has no home here is listed; and the import sends
// back the digest the preview showed, so a folder that changed in between can be refused.

const plan = {
  digest: "abc123",
  personas: [
    {
      id: "desk-helper",
      name: "Desk Helper",
      tagline: "Everyday Work",
      instructions: "You are Desk Helper, Everyday Work.",
      tools: ["files", "web"],
      skills: ["find-a-doc"],
      remote: null,
      problems: [],
      notes: ["Its skill `nope` isn't coming across, so it goes without."],
    },
    {
      id: "Bad Id",
      name: "Broken",
      tagline: "",
      instructions: "",
      tools: [],
      skills: [],
      remote: null,
      problems: ["“Bad Id” isn't an id an assistant can have here."],
      notes: [],
    },
  ],
  skills: [
    {
      name: "find-a-doc",
      title: "Find a document",
      description: "Search the sources.",
      instructions: "Search first.",
      tools: [],
      unmapped: ["google-drive/search_files"],
      problems: [],
    },
  ],
  left_behind: ["Channels: they decide which groups of people may reach which bots, and this app has one person."],
};

let calls: { url: string; body: any }[] = [];

beforeEach(() => {
  calls = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      calls.push({ url: String(url), body });
      const reply = (v: any) => ({ ok: true, json: async () => v }) as Response;
      if (String(url).endsWith("/v1/import/package/preview")) return reply({ ok: true, plan });
      if (String(url).endsWith("/v1/import/package"))
        return reply({
          ok: true,
          skills: ["find-a-doc"],
          consent: [
            {
              id: "desk-helper",
              name: "Desk Helper",
              description: "Everyday Work",
              tools: ["files", "web"],
              risk: [],
              connectors: true,
              mcp: [],
              messaging: false,
              recommended_mode: "interactive",
              recommended_models: [],
              source: "the agent package at /pkg",
            },
          ],
          personas: [],
        });
      if (String(url).includes("/v1/sessions")) return reply({ sessions: [] });
      return reply({ personas: [] });
    }),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("PersonasTab ▸ agent package", () => {
  it("reads the package into a plan and writes nothing until Import", async () => {
    render(<PersonasTab />);
    fireEvent.change(screen.getByTestId("install-mode"), { target: { value: "package" } });
    fireEvent.change(screen.getByTestId("install-source"), { target: { value: " /pkg " } });
    fireEvent.click(screen.getByTestId("install-go"));

    await waitFor(() => expect(screen.getByTestId("package-plan")).toBeTruthy());
    expect(calls.filter((c) => c.url.includes("/v1/import/package") && !c.url.endsWith("/preview"))).toHaveLength(0);
    expect(screen.getByTestId("plan-persona-desk-helper").textContent).toContain("goes without");
    expect(screen.getByTestId("plan-skill-find-a-doc").textContent).toContain("google-drive/search_files");
    expect(screen.getByTestId("plan-refused").textContent).toContain("Broken:");
    expect(screen.getByTestId("plan-left-behind").textContent).toContain("Channels");
    expect(screen.getByTestId("plan-import").textContent).toBe("Import 1 assistant and 1 skill");

    fireEvent.click(screen.getByTestId("plan-import"));
    await waitFor(() => expect(screen.getByText(/the assistants are off until you enable them/)).toBeTruthy());
    const imported = calls.find((c) => c.url.endsWith("/v1/import/package"));
    expect(imported?.body).toEqual({ dir: "/pkg", digest: "abc123" });
    expect(screen.queryByTestId("package-plan")).toBeNull();
    expect(screen.getByText(/From the agent package at \/pkg/)).toBeTruthy();
  });
});
