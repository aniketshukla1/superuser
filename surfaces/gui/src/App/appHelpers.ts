import { CHAT_COMFORT_W } from "../components/ResizeHandle";
import { type IntroMode, pickIntro } from "../components/LaunchFilm";
import type { TodoItem, SessionInfo, Item, ApprovalDecision } from "../types";
import type { RecentWorkspace } from "../api";

/** The right panel's floor, shared by its own clamp and the nav's reserve. */
export const PANEL_MIN_W = 360;
/** The panel's width before anyone drags it. It was 560 — wider than the chat it sat
 *  beside at the default window — for an inspector that is empty most of the time. 400
 *  holds a file preview and the activity log; dragging wider is still one gesture. */
export const PANEL_DEFAULT_W = 400;
/** The nav's default width; `usePanelWidth` below falls back to the same number. */
export const NAV_DEFAULT_W = 240;
/** The window width at which the panel opens by itself: the nav, a comfortable chat and a
 *  default-width panel, side by side. Derived, so moving any of the three moves this. */
export const RAIL_MIN_W = NAV_DEFAULT_W + CHAT_COMFORT_W + PANEL_DEFAULT_W;
/** When the app is ready after the reel has finished, how long the final frame holds
 *  before leaving — enough to see the last reading turn green. */
export const SPLASH_SETTLE_MS = 450;
export let launchIntro: IntroMode | null = null;
/** Which cut of the launch film plays, decided once per page load: StrictMode runs state
 *  initialisers twice, and deciding also records that the intro has been seen. */
export const introForThisLaunch = (): IntroMode => (launchIntro ??= pickIntro());

export const newId = () =>
  (crypto as any).randomUUID ? crypto.randomUUID().slice(0, 12) : Math.random().toString(36).slice(2, 14);

// Tools whose success means a new/changed file should show up under Artifacts right away.
// Tools whose result the panel should show immediately, without waiting for the turn
// to end. The document tools were missing, so an agent that edited a spreadsheet left
// the open panel showing the old contents until the user reselected the file.
export const FILE_WRITE_TOOLS = new Set([
  "write_file",
  "apply_patch",
  "apply_unified_diff",
  "replace_in_file",
  "create_file",
  "edit_file",
  "sheet_new",
  "sheet_write",
  "doc_write",
  "slides_write",
  "slides_edit",
  "doc_edit",
]);

// Models sometimes pass todo items as bare strings instead of {content, status} objects (the
// backend tool normalizes them the same way; the GUI reads the raw proposal args, so mirror it).
export function normalizeTodos(raw: unknown): TodoItem[] {
  if (!Array.isArray(raw)) return [];
  const statuses = new Set(["pending", "in_progress", "done"]);
  return raw.map((entry: any) => {
    if (entry && typeof entry === "object") {
      const status = entry.status === "completed" ? "done" : entry.status; // common model alias
      return {
        content: String(entry.content ?? ""),
        status: statuses.has(status) ? status : "pending",
      };
    }
    return { content: String(entry ?? ""), status: "pending" as const };
  });
}

// Fallbacks used only before the persona list loads (the in-component, family-aware
// needsWorkspace/gatesWorkspace consult the real persona once available).
// The general assistant's id, matching `personas::DEFAULT_PERSONA_ID` in Rust.
//
// It used to be "cowork", and the rename left every condition behind. `agent` was
// initialised to "cowork", the surface-visibility guard fell back to it, and three
// features were gated on `agent === "cowork"` — an id that is in no roster this app
// serves, so those three had simply stopped happening. A comparison against a value
// nothing can hold is not dead code you can see; it reads as a working feature.
export const GENERAL_ASSISTANT = "assistant";
export const needsWorkspaceFallback = (a: string) => a === "coder" || a === GENERAL_ASSISTANT;
export const gatesWorkspaceFallback = (a: string) => a === "coder";
export const LAST_SESSION_KEY = "assistant:last-session-by-agent:v1";
export const NAV_COLLAPSED_KEY = "assistant:nav-collapsed:v1";

export type LastSession = { sessionId: string; workspace: string; updatedAt: number };

export function readLastSessions(): Record<string, LastSession> {
  try {
    const raw = localStorage.getItem(LAST_SESSION_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

export function rememberLastSession(agent: string, sessionId: string, workspace: string | null) {
  if (!agent || !sessionId) return;
  try {
    const all = readLastSessions();
    all[agent] = { sessionId, workspace: workspace || "", updatedAt: Date.now() };
    localStorage.setItem(LAST_SESSION_KEY, JSON.stringify(all));
  } catch {
    /* localStorage may be unavailable; session restore is best effort. */
  }
}

export function sessionTs(s: SessionInfo): number {
  return Date.parse(s.updated_at || "") || Number(s.updated_at) || 0;
}

export function resumeTargetForAgent(agent: string, sessions: SessionInfo[]): LastSession | null {
  const remembered = readLastSessions()[agent];
  if (remembered?.sessionId) {
    const live = sessions.find((s) => s.session_id === remembered.sessionId && s.agent === agent);
    if (live || remembered.workspace) {
      return {
        sessionId: remembered.sessionId,
        workspace: live?.workspace ?? remembered.workspace ?? "",
        updatedAt: live ? sessionTs(live) : remembered.updatedAt,
      };
    }
  }
  const recent = sessions
    .filter((s) => s.agent === agent && s.session_id && !s.session_id.startsWith("__"))
    .sort((a, b) => sessionTs(b) - sessionTs(a))[0];
  return recent ? { sessionId: recent.session_id, workspace: recent.workspace || "", updatedAt: sessionTs(recent) } : null;
}

export function fallbackWorkspace(current: string | null, projects: RecentWorkspace[]): string {
  if (current) return current;
  const existing = projects.find((p) => p.exists);
  return existing?.path || projects[0]?.path || "";
}

export function updateLastTool(
  items: Item[],
  name: string,
  status: string,
  preview?: string,
  hidden?: number,
  standingRule?: string,
): Item[] {
  const copy = [...items];
  for (let i = copy.length - 1; i >= 0; i--) {
    const it = copy[i];
    if (it.kind === "tool" && it.name === name && it.status === "…") {
      copy[i] = {
        ...it,
        status,
        preview,
        ...(hidden ? { hidden } : {}),
        ...(standingRule ? { standingRule } : {}),
      };
      break;
    }
  }
  return copy;
}

export function resolveLastApproval(items: Item[], decision: ApprovalDecision): Item[] {
  const copy = [...items];
  for (let i = copy.length - 1; i >= 0; i--) {
    const it = copy[i];
    if (it.kind === "approval" && !it.resolved) {
      copy[i] = { ...it, resolved: decision };
      break;
    }
  }
  return copy;
}

export function resolveLastDirReq(items: Item[], resolved: "granted" | "denied"): Item[] {
  const copy = [...items];
  for (let i = copy.length - 1; i >= 0; i--) {
    const it = copy[i];
    if (it.kind === "dirreq" && !it.resolved) {
      copy[i] = { ...it, resolved };
      break;
    }
  }
  return copy;
}

export function resolveLastPlan(items: Item[], resolved: "approved" | "rejected"): Item[] {
  const copy = [...items];
  for (let i = copy.length - 1; i >= 0; i--) {
    const it = copy[i];
    if (it.kind === "planreq" && !it.resolved) {
      copy[i] = { ...it, resolved };
      break;
    }
  }
  return copy;
}

export function resolveLastQuestion(items: Item[], answer: string): Item[] {
  const copy = [...items];
  for (let i = copy.length - 1; i >= 0; i--) {
    const it = copy[i];
    if (it.kind === "question" && !it.resolved) {
      copy[i] = { ...it, resolved: answer };
      break;
    }
  }
  return copy;
}
