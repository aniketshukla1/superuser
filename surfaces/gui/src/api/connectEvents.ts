import type { Automation, AutomationRun } from "./automations";
import { fetch, httpBase, jsonBody, jsonOf, listOf, openWebSocket, wsBase } from "./neverRejectingJson";

// The profile (how the person wants to be worked with), the app-wide event stream, and the
// automations list.
//
// The profile is the third memory store: the journal keeps facts on a timeline, knowledge
// keeps topics, and this keeps behaviour, which fades over time (see `core/profile.rs`).

export type FacetClass = "identity" | "style" | "tooling" | "veto" | "goal" | "channel";

export interface Facet {
  class: FacetClass;
  /** What the belief is about. Two beliefs with the same key compete for it. */
  key: string;
  value: string;
  /** Active facets shape answers; provisional ones are only being watched. */
  state: "active" | "provisional";
  cue: "explicit" | "structural" | "behavioral";
  score: number;
  observations: number;
  last_seen: number;
  /** What was said that led to it. */
  evidence: string;
  pinned: boolean;
  /** Comes straight from the profile form; change it there. */
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
  /** Where PROFILE.md lives on disk. */
  path: string;
}

const FORM_FIELDS = ["name", "call_me", "role", "about", "tone", "interests", "likes", "dislikes", "goals"] as const;
export const EMPTY_FORM: ProfileForm = Object.fromEntries(FORM_FIELDS.map((field) => [field, ""])) as unknown as ProfileForm;

const profileUrl = (path = "") => `${httpBase()}/v1/profile${path}`;

export async function getProfile(): Promise<ProfileState> {
  const res = await fetch(profileUrl());
  if (!res.ok) return { form: EMPTY_FORM, facets: [], forgotten: [], learning_enabled: false, path: "" };
  const body = await jsonOf(res);
  return {
    form: { ...EMPTY_FORM, ...(body.form ?? {}) },
    facets: body.facets ?? [],
    forgotten: body.forgotten ?? [],
    learning_enabled: body.learning_enabled ?? false,
    path: body.path ?? "",
  };
}

/** POST to the profile and report whether the server said ok. */
async function profilePost(path: string, body: unknown): Promise<{ took: boolean; reply: Record<string, any> }> {
  const res = await fetch(profileUrl(path), jsonBody("POST", body));
  const reply = await jsonOf(res);
  return { took: res.ok && reply.ok === true, reply };
}

export async function saveProfileForm(form: ProfileForm): Promise<boolean> {
  return (await profilePost("/form", form)).took;
}

/** Protect a belief from fading, or let it fade again. */
export async function pinFacet(f: Facet, pinned: boolean): Promise<Facet[] | null> {
  const { reply } = await profilePost("/pin", { class: f.class, key: f.key, value: f.value, pinned });
  return reply.ok ? reply.facets : null;
}

/** Drop a belief and stop it being learned again; `forget: false` lifts that. */
export async function forgetFacet(f: { class: FacetClass; key: string; value: string }, forget = true): Promise<Facet[] | null> {
  const { reply } = await profilePost("/forget", { class: f.class, key: f.key, value: f.value, forget });
  return reply.ok ? reply.facets : null;
}

export async function setProfileLearning(enabled: boolean): Promise<boolean> {
  return (await profilePost("/learning", { enabled })).took;
}

const automationsUrl = (path = "") => `${httpBase()}/v1/automations${path}`;

export async function getAutomations(): Promise<Automation[]> {
  return listOf(await fetch(automationsUrl()), "tasks");
}

/** Sent after an automation change the sidebar should show at once (seen, created, deleted);
 *  its regular poll covers everything else. */
export const AUTOMATIONS_CHANGED = "assistant:automations-changed";
export function announceAutomationsChanged() {
  window.dispatchEvent(new CustomEvent(AUTOMATIONS_CHANGED));
}

type AppEvent = { type: string; data?: Record<string, unknown> };

/** The app-wide event stream (/ws/events): pushes from the server that belong to no session,
 *  today automation_run_started (the UX-026 toast). It reconnects quietly while the app is
 *  open; the function it returns stops it for good. */
export function connectEvents(onEvent: (msg: AppEvent) => void): () => void {
  let socket: WebSocket | null = null;
  let retry: number | null = null;
  let stopped = false;
  const connect = () => {
    if (stopped) return;
    socket = openWebSocket(`${wsBase()}/ws/events`);
    socket.onmessage = (message) => {
      // A bad frame, or a handler that throws on one, must not break the stream.
      try {
        onEvent(JSON.parse(message.data));
      } catch {
        // Skipped.
      }
    };
    socket.onclose = () => {
      if (!stopped) retry = window.setTimeout(connect, 5000);
    };
  };
  connect();
  return () => {
    stopped = true;
    if (retry !== null) window.clearTimeout(retry);
    socket?.close();
  };
}

/** Move the automation's "seen" mark forward, which clears its unseen-runs badge (UX-023). */
export async function markAutomationSeen(id: string): Promise<{ ok: boolean }> {
  return (await fetch(automationsUrl(`/${id}/seen`), { method: "POST" })).json();
}

export type NewAutomation = {
  title: string;
  instructions: string;
  cron?: string;
  fire_at?: string;
  timezone?: string;
  /** Standing grants (§25). The surface that creates the automation showed them, so submitting
   *  is the consent; the server keeps only write entries bound to a target. */
  permissions?: { tool: string; target: string; access: "read" | "write" }[];
  /** The conversation it was set up from. Each run's answer is written back there, so the
   *  person can follow up where they asked. */
  session_id?: string;
};

export async function createAutomation(payload: NewAutomation): Promise<{ ok: boolean; error?: string; task?: Automation }> {
  return (await fetch(automationsUrl(), jsonBody("POST", payload))).json();
}

export async function getAutomation(id: string): Promise<{ task: Automation; runs: AutomationRun[] }> {
  return (await fetch(automationsUrl(`/${encodeURIComponent(id)}`))).json();
}
