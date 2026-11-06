import type { RecentWorkspace } from "../api";
import { type IntroMode, pickIntro } from "../components/LaunchFilm";
import { CHAT_COMFORT_W } from "../components/ResizeHandle";
import type { ApprovalDecision, Item, SessionInfo, TodoItem } from "../types";

// Constants and small pure helpers the app model shares.

/** The narrowest the right panel gets; its own clamp and the nav's reserve both use it. */
export const PANEL_MIN_W = 360;
/** The panel's width until someone drags it. It used to be 560, wider than the chat beside it
 *  in the default window, for an inspector that is mostly empty; 400 fits a file preview and
 *  the activity log, and widening it is still one drag. */
export const PANEL_DEFAULT_W = 400;
/** The nav's width by default; `usePanelWidth` falls back to the same value. */
export const NAV_DEFAULT_W = 240;
/** The window width at which the panel opens on its own: the nav, a comfortable chat and a
 *  default panel side by side. Worked out from the three, so changing one moves it. */
export const RAIL_MIN_W = NAV_DEFAULT_W + CHAT_COMFORT_W + PANEL_DEFAULT_W;
/** When the app is ready only after the film has ended, how long the last frame stays: long
 *  enough to see the final reading turn green. */
export const SPLASH_SETTLE_MS = 450;

export let launchIntro: IntroMode | null = null;
/** Which cut of the launch film plays, settled once per page load: StrictMode runs state
 *  initialisers twice, and settling it also records that the intro was seen. */
export const introForThisLaunch = (): IntroMode => (launchIntro ??= pickIntro());

/** A short random id: twelve characters of a UUID where the browser has one. */
export function newId(): string {
  if (typeof (crypto as any).randomUUID === "function") return crypto.randomUUID().slice(0, 12);
  return Math.random().toString(36).slice(2, 14);
}

/** Tools whose success should show in the panel at once, without waiting for the turn to end:
 *  a new or changed file belongs under Artifacts straight away. The document tools were once
 *  missing, so a spreadsheet the agent edited stayed stale in the open panel until reselected. */
export const FILE_WRITE_TOOLS = new Set([
  "apply_patch", "apply_unified_diff", "create_file", "doc_edit", "doc_write", "edit_file",
  "replace_in_file", "sheet_new", "sheet_write", "slides_edit", "slides_write", "write_file",
]);

const TODO_STATES = new Set<TodoItem["status"]>(["pending", "in_progress", "done"]);

/** The todo list from a tool call's raw arguments. Models sometimes send bare strings rather
 *  than {content, status}, and "completed" for done; the server tidies the same way, but the
 *  GUI reads the proposal as sent, so it does too. */
export function normalizeTodos(raw: unknown): TodoItem[] {
  if (!Array.isArray(raw)) return [];
  return raw.map((entry: any): TodoItem => {
    if (entry === null || typeof entry !== "object") return { content: String(entry ?? ""), status: "pending" };
    const status = entry.status === "completed" ? "done" : entry.status;
    return { content: String(entry.content ?? ""), status: TODO_STATES.has(status) ? status : "pending" };
  });
}

// Used only until the assistant list has loaded; after that the family-aware checks read the
// real assistant. `GENERAL_ASSISTANT` matches `personas::DEFAULT_PERSONA_ID` on the server.
//
// It used to be "cowork", and the rename left the conditions behind: `agent` started as
// "cowork", the surface-visibility guard fell back to it, and three features waited on
// `agent === "cowork"`, an id no roster in this app has. So those three quietly stopped. A
// comparison against a value nothing can hold doesn't look dead; it looks like a feature.
export const GENERAL_ASSISTANT = "assistant";
export const needsWorkspaceFallback = (a: string) => a === "coder" || a === GENERAL_ASSISTANT;
export const gatesWorkspaceFallback = (a: string) => a === "coder";
export const LAST_SESSION_KEY = "assistant:last-session-by-agent:v1";
export const NAV_COLLAPSED_KEY = "assistant:nav-collapsed:v1";

export type LastSession = { sessionId: string; workspace: string; updatedAt: number };

/** The last session per assistant, from storage; empty when there's none or it can't be read. */
export function readLastSessions(): Record<string, LastSession> {
  try {
    const stored = localStorage.getItem(LAST_SESSION_KEY);
    return stored ? JSON.parse(stored) : {};
  } catch {
    return {};
  }
}

export function rememberLastSession(agent: string, sessionId: string, workspace: string | null) {
  if (!agent || !sessionId) return;
  try {
    const next = { ...readLastSessions(), [agent]: { sessionId, workspace: workspace || "", updatedAt: Date.now() } };
    localStorage.setItem(LAST_SESSION_KEY, JSON.stringify(next));
  } catch {
    // No storage: picking up where you left off is a nicety, not a promise.
  }
}

/** When a session was last active, in ms; the server may send an ISO date or a number. */
export function sessionTs(s: SessionInfo): number {
  return Date.parse(s.updated_at || "") || Number(s.updated_at) || 0;
}

/** Where an assistant should pick up: the session last opened with it, if that still exists
 *  or at least has a folder, otherwise its most recent ordinary session. */
export function resumeTargetForAgent(agent: string, sessions: SessionInfo[]): LastSession | null {
  const remembered = readLastSessions()[agent];
  if (remembered?.sessionId) {
    const live = sessions.find((s) => s.agent === agent && s.session_id === remembered.sessionId);
    if (live) return { sessionId: remembered.sessionId, workspace: live.workspace ?? remembered.workspace ?? "", updatedAt: sessionTs(live) };
    if (remembered.workspace) return { sessionId: remembered.sessionId, workspace: remembered.workspace, updatedAt: remembered.updatedAt };
  }
  let newest: SessionInfo | null = null;
  for (const s of sessions) {
    // Ids starting "__" belong to automation runs and the like, not to conversations.
    if (s.agent !== agent || !s.session_id || s.session_id.startsWith("__")) continue;
    if (!newest || sessionTs(s) > sessionTs(newest)) newest = s;
  }
  return newest ? { sessionId: newest.session_id, workspace: newest.workspace || "", updatedAt: sessionTs(newest) } : null;
}

export function fallbackWorkspace(current: string | null, projects: RecentWorkspace[]): string {
  if (current) return current;
  const firstExisting = projects.find((p) => p.exists);
  return firstExisting?.path || projects[0]?.path || "";
}

type OfKind<K extends Item["kind"]> = Extract<Item, { kind: K }>;

/** A new list with the newest item of `kind` that passes `pick` replaced by `change(item)`. */
function changeNewest<K extends Item["kind"]>(items: Item[], kind: K, pick: (it: OfKind<K>) => boolean, change: (it: OfKind<K>) => Item): Item[] {
  const next = [...items];
  for (let at = next.length - 1; at >= 0; at--) {
    const it = next[at];
    if (it.kind !== kind || !pick(it as OfKind<K>)) continue;
    next[at] = change(it as OfKind<K>);
    break;
  }
  return next;
}

/** Settle the newest still-running call of the tool `name`. `preview` is always written; the
 *  privacy count and the standing rule only when there is one. */
export function updateLastTool(items: Item[], name: string, status: string, preview?: string, hidden?: number, standingRule?: string): Item[] {
  return changeNewest(items, "tool", (t) => t.name === name && t.status === "…", (t) => {
    const settled: OfKind<"tool"> = { ...t, status, preview };
    if (hidden) settled.hidden = hidden;
    if (standingRule) settled.standingRule = standingRule;
    return settled;
  });
}

const unresolved = (it: { resolved?: unknown }) => !it.resolved;

export const resolveLastApproval = (items: Item[], decision: ApprovalDecision): Item[] =>
  changeNewest(items, "approval", unresolved, (it) => ({ ...it, resolved: decision }));

export const resolveLastDirReq = (items: Item[], resolved: "granted" | "denied"): Item[] =>
  changeNewest(items, "dirreq", unresolved, (it) => ({ ...it, resolved }));

export const resolveLastPlan = (items: Item[], resolved: "approved" | "rejected"): Item[] =>
  changeNewest(items, "planreq", unresolved, (it) => ({ ...it, resolved }));

export const resolveLastQuestion = (items: Item[], answer: string): Item[] =>
  changeNewest(items, "question", unresolved, (it) => ({ ...it, resolved: answer }));

export const resolveLastForm = (items: Item[], answers: string): Item[] =>
  changeNewest(items, "form", unresolved, (it) => ({ ...it, resolved: answers }));
