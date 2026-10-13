import { vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Sidebar } from "../Sidebar";
import type { SessionInfo } from "../../types";

// Hermetic fetch stub routing by URL substring + method; records calls for POST assertions.
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

export const PERSONAS = {
  personas: [
    { id: "cowork", name: "Superuser", icon: "cowork", tagline: "general assistant", family: "knowledge", enabled: true, surfaced: true, default: true },
    { id: "ops", name: "Ops", icon: "ops", tagline: "incidents, runbooks", family: "code", enabled: true, surfaced: true, default: false },
    { id: "code", name: "Code", icon: "code", tagline: "repository work", family: "code", enabled: true, surfaced: true, default: false },
    { id: "secret", name: "Disabled One", icon: "cowork", tagline: "off", family: "knowledge", enabled: false, surfaced: false, default: false },
  ],
};

export const SESSIONS: SessionInfo[] = [
  { session_id: "s-ops-1", title: "incident watch", workspace: "/w", agent: "ops", model: "m", mode: "interactive", updated_at: "2026-06-29", messages: 2 },
  { session_id: "s-cowork-1", title: "hi there", workspace: "", agent: "cowork", model: "m", mode: "interactive", updated_at: "2026-06-29", messages: 1 },
];

export const baseProps = {
  agent: "cowork",
  workspace: "",
  surfaces: { cowork: true, chat: false, code: false },
  sessions: SESSIONS,
  projects: [],
  activeSession: "s-cowork-1",
  onSwitchAgent: vi.fn(),
  onNewSession: vi.fn(),
  onSelectSession: vi.fn(),
  onNewProject: vi.fn(),
  onNewInProject: vi.fn(),
  onRenameSession: vi.fn(),
  onDeleteSession: vi.fn(),
  onArchiveSession: vi.fn(),
  onTogglePin: vi.fn(),
  onManage: vi.fn(),
  onOpenPersona: vi.fn(),
  onManagePersonas: vi.fn(),
  onOpenScheduled: vi.fn(),
  onOpenAutomation: vi.fn(),
  onNewAutomation: vi.fn(),
  onOpenIntegrations: vi.fn(),
  onOpenAudit: vi.fn(),
  onOpenInbox: vi.fn(),
  onOpenBrain: vi.fn(),
  scheduledActive: false,
  integrationsActive: false,
  auditActive: false,
  inboxActive: false,
  brainActive: false,
};
export const renderSidebar = async () => {
  stubFetch([
    { match: "/v1/personas", method: "GET", json: PERSONAS },
    { match: "/v1/settings", method: "GET", json: { nav_layout: "flat" } },
  ]);
  render(<Sidebar {...baseProps} />);
  await screen.findByLabelText("Group and filter conversations");
};

// The control filters the persona list to one capability: an assistant talks, a
// workbench runs a shell. PERSONAS has both (cowork/ops are assistants by
// default, and a workbench is added here), so the control is offered.
export const MIXED = {
  personas: [
    ...PERSONAS.personas,
    { id: "bench", name: "Bench", icon: "code", tagline: "repo work", family: "code", kind: "workbench", enabled: true, surfaced: true, default: false },
  ],
};

// Under Code the folder is what you navigate by, so the list is projects rather
// than personas — the same project used to appear under every workbench persona
// that had touched it, and finding work meant remembering which assistant you
// happened to use last.
export const BENCH_SESSIONS: SessionInfo[] = [
  ...SESSIONS,
  { session_id: "s-bench-1", title: "refactor the parser", workspace: "/w/aegis", agent: "bench", model: "m", mode: "interactive", updated_at: "2026-06-30", messages: 3 },
];

// Folders used to be a Set of independent toggles XOR'd against a per-folder default,
// while the persona headers next door were a plain accordion. Two lists, same gesture,
// different behaviour — and because the default moved with the workspace, a folder
// could open or shut itself with no click involved.
export const THREE_FOLDERS: SessionInfo[] = [
  ...SESSIONS,
  { session_id: "s-a", title: "refactor the parser", workspace: "/w/aegis", agent: "bench", model: "m", mode: "interactive", updated_at: "2026-06-30", messages: 3 },
  { session_id: "s-b", title: "wire the loader", workspace: "/w/borealis", agent: "bench", model: "m", mode: "interactive", updated_at: "2026-06-29", messages: 2 },
  { session_id: "s-c", title: "trim the config", workspace: "/w/cirrus", agent: "bench", model: "m", mode: "interactive", updated_at: "2026-06-28", messages: 2 },
];

export const openFolders = () =>
  [...screen.getByTestId("project-first").querySelectorAll("[aria-expanded]")]
    .filter((el) => el.getAttribute("aria-expanded") === "true")
    .map((el) => el.textContent);

export async function codeTab(props: Partial<typeof baseProps> = {}) {
  stubFetch([
    { match: "/v1/personas", method: "GET", json: MIXED },
    { match: "/v1/settings", method: "GET", json: { nav_layout: "grouped" } },
  ]);
  render(<Sidebar {...baseProps} sessions={THREE_FOLDERS} agent="bench" {...props} />);
  await screen.findByTestId("surface-tab-code");
  fireEvent.click(screen.getByTestId("surface-tab-code"));
}
