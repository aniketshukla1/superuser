import { vi } from "vitest";
import { fireEvent, screen } from "@testing-library/react";

// SKILLS-SPEC §5/§6 GUI — Settings ▸ Skills: list + badges, the switch, delete, and the two
// doors (install from a link / doorway-to-conversation). The write form, file import, editing
// and "Show folder" are gone until the server serves them.

export type Call = { url: string; method: string; body: any };

export function stubFetch(routes: { match: string; method?: string; json: any }[]) {
  const calls: Call[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    const method = (init?.method || "GET").toUpperCase();
    calls.push({ url, method, body: init?.body ? JSON.parse(String(init.body)) : undefined });
    for (const r of routes) {
      if (url.includes(r.match) && (!r.method || r.method === method)) {
        return { ok: true, json: async () => r.json } as Response;
      }
    }
    return { ok: true, json: async () => ({}) } as Response;
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

export const ROW = {
  name: "weekly-report",
  description: "Monday status report",
  instructions: "1. Collect updates\n2. Write it up",
  scope: "global",
  source: "local",
  enabled: true,
  path: "/skills/weekly-report",
};

export const UPLOADED_ROW = {
  ...ROW,
  name: "greet",
  description: "says hello",
  source: "uploaded",
  enabled: false,
};

export const LIST = { skills: [ROW, UPLOADED_ROW] };

export const PREVIEW = {
  name: "weekly",
  description: "Monday status report",
  instructions: "1. Collect updates\n2. Write it up",
  allowed_tools: [],
  source: "owner/repo",
  url: "https://github.com/owner/repo/blob/main/SKILL.md",
  replaces_existing: false,
};

// The single add-action: open the "Add skill" menu, take the link door (SKILLS-SPEC §5).
export const openLinkDoor = async () => {
  fireEvent.click(await screen.findByRole("button", { name: /Add skill/ }));
  fireEvent.click(screen.getByText("Install from a link"));
};

// Every request the page may make, as the server routes them (`api_server.rs`). A request
// outside this list reaches a path nothing serves — the fork's page made five of those.
export const SERVED: [string, RegExp][] = [
  ["GET", /^\/v1\/skills$/],
  ["POST", /^\/v1\/skills\/preview$/],
  ["POST", /^\/v1\/skills\/install$/],
  ["PATCH", /^\/v1\/skills\/[^/]+$/],
  ["DELETE", /^\/v1\/skills\/[^/]+$/],
];
export const served = (c: Call) => {
  const path = c.url.slice(c.url.indexOf("/v1/")).split("?")[0];
  return SERVED.some(([method, re]) => method === c.method && re.test(path));
};
