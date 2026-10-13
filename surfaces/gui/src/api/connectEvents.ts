import { fetch, httpBase, jsonOf, listOf, openWebSocket, wsBase } from "./neverRejectingJson";
import type { Automation, AutomationRun } from "./automations";

// -- Profile: how you want to be worked with ---------------------------------
// The third memory store. The journal holds facts on a timeline and knowledge
// holds topics; this holds behaviour, and it decays — see `core/profile.rs`.

export type FacetClass = "identity" | "style" | "tooling" | "veto" | "goal" | "channel";

export interface Facet {
  class: FacetClass;
  /** The slot this competes for. Two beliefs with the same key are rivals. */
  key: string;
  value: string;
  /** Active facets steer answers; provisional ones are only tracked. */
  state: "active" | "provisional";
  score: number;
  cue: "explicit" | "structural" | "behavioral";
  observations: number;
  last_seen: number;
  /** What was said that put it here. */
  evidence: string;
  pinned: boolean;
  /** Straight from the profile form below — change it there, not here. */
  from_form: boolean;
}

export interface ProfileForm {
  name: string;
  call_me: string;
  role: string;
  about: string;
  tone: string;
  interests: string;
  likes: string;
  dislikes: string;
  goals: string;
}

export interface ProfileState {
  form: ProfileForm;
  facets: Facet[];
  forgotten: { class: FacetClass; key: string; value: string; at: number }[];
  learning_enabled: boolean;
  /** Where PROFILE.md is on disk. */
  path: string;
}

export const EMPTY_FORM: ProfileForm = {
  name: "",
  call_me: "",
  role: "",
  about: "",
  tone: "",
  interests: "",
  likes: "",
  dislikes: "",
  goals: "",
};

export async function getProfile(): Promise<ProfileState> {
  const res = await fetch(`${httpBase()}/v1/profile`);
  if (!res.ok) {
    return { form: EMPTY_FORM, facets: [], forgotten: [], learning_enabled: false, path: "" };
  }
  const j = await jsonOf(res);
  return {
    form: { ...EMPTY_FORM, ...(j.form ?? {}) },
    facets: j.facets ?? [],
    forgotten: j.forgotten ?? [],
    learning_enabled: j.learning_enabled ?? false,
    path: j.path ?? "",
  };
}

export async function saveProfileForm(form: ProfileForm): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/profile/form`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(form),
  });
  return res.ok && (await jsonOf(res)).ok === true;
}

/** Shield a belief from decay, or let it fade again. */
export async function pinFacet(f: Facet, pinned: boolean): Promise<Facet[] | null> {
  const res = await fetch(`${httpBase()}/v1/profile/pin`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ class: f.class, key: f.key, value: f.value, pinned }),
  });
  const j = await jsonOf(res);
  return j.ok ? j.facets : null;
}

/** Drop a belief and block it from being learned again (`forget: false` lifts it). */
export async function forgetFacet(
  f: { class: FacetClass; key: string; value: string },
  forget = true,
): Promise<Facet[] | null> {
  const res = await fetch(`${httpBase()}/v1/profile/forget`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ class: f.class, key: f.key, value: f.value, forget }),
  });
  const j = await jsonOf(res);
  return j.ok ? j.facets : null;
}

export async function setProfileLearning(enabled: boolean): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/profile/learning`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ enabled }),
  });
  return res.ok && (await jsonOf(res)).ok === true;
}

export async function getAutomations(): Promise<Automation[]> {
  const res = await fetch(`${httpBase()}/v1/automations`);
  return listOf(res, "tasks");
}

// Fired after any automation mutation the sidebar should reflect immediately
// (mark-seen, create, delete) — its poll covers the rest.
export const AUTOMATIONS_CHANGED = "assistant:automations-changed";
export function announceAutomationsChanged() {
  window.dispatchEvent(new CustomEvent(AUTOMATIONS_CHANGED));
}

/** App-wide event stream (/ws/events): session-independent server pushes — today
 * automation_run_started (the UX-026 toast). Quietly reconnects while the app is
 * open; the returned cleanup stops it for good. */
export function connectEvents(
  onEvent: (msg: { type: string; data?: Record<string, unknown> }) => void
): () => void {
  let ws: WebSocket | null = null;
  let timer: number | null = null;
  let closed = false;
  const open = () => {
    if (closed) return;
    ws = openWebSocket(`${wsBase()}/ws/events`);
    ws.onmessage = (e) => {
      try {
        onEvent(JSON.parse(e.data));
      } catch {
        /* malformed frame — ignore */
      }
    };
    ws.onclose = () => {
      if (!closed) timer = window.setTimeout(open, 5000);
    };
  };
  open();
  return () => {
    closed = true;
    if (timer !== null) window.clearTimeout(timer);
    ws?.close();
  };
}

/** Advance the automation's seen mark — clears its unseen-runs badge (UX-023). */
export async function markAutomationSeen(id: string): Promise<{ ok: boolean }> {
  const res = await fetch(`${httpBase()}/v1/automations/${id}/seen`, { method: "POST" });
  return res.json();
}

export async function createAutomation(payload: {
  title: string;
  instructions: string;
  cron?: string;
  fire_at?: string;
  timezone?: string;
  // §25 standing grants (the creating surface rendered them; submit IS the consent).
  // Only target-bound write entries survive server-side validation.
  permissions?: { tool: string; target: string; access: "read" | "write" }[];
  // The conversation this was set up from. Each run's answer is written back here
  // so the user can follow up on it where they asked for it.
  session_id?: string;
}): Promise<{ ok: boolean; error?: string; task?: Automation }> {
  const res = await fetch(`${httpBase()}/v1/automations`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  return res.json();
}

export async function getAutomation(id: string): Promise<{ task: Automation; runs: AutomationRun[] }> {
  const res = await fetch(`${httpBase()}/v1/automations/${encodeURIComponent(id)}`);
  return res.json();
}
