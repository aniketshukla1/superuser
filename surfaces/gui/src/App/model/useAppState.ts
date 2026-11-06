import { useCallback, useEffect, useRef, useState } from "react";
import type { RecentWorkspace, SurfaceVisibility, WorkspaceCommandTrust } from "../../api";
import type { SetTab } from "../../components/SettingsView";
import { useRoute, type RouteSurface } from "../../route";
import { TRACE, trace } from "../../trace";
import type { Item, SessionInfo, SessionUsage, TodoItem } from "../../types";
import { emptyUsage } from "../../usage";
import { GENERAL_ASSISTANT, newId } from "../appHelpers";

/** Text that socket handlers, built once per socket, also need to read live: the value for
 *  rendering, a ref with the same value for those handlers, and a setter that moves both. */
function useMirrored(initial: string) {
  const [value, setValue] = useState(initial);
  const ref = useRef(initial);
  const set = (next: string | ((current: string) => string)) => {
    ref.current = typeof next === "function" ? next(ref.current) : next;
    setValue(ref.current);
  };
  return [value, ref, set] as const;
}

/** The app's core state: folder, assistant, model, transcript, sessions and route. */
export function useAppState() {
  // Where work happens.
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [branch, setBranch] = useState<string | null>(null);
  const [projects, setProjects] = useState<RecentWorkspace[]>([]);
  const [showGate, setShowGate] = useState(false);
  const [gateCreate, setGateCreate] = useState(false);
  const [workspaceTrustRequest, setWorkspaceTrustRequest] = useState<WorkspaceCommandTrust | null>(null);

  // Who answers, and with what.
  const [agent, setAgent] = useState(GENERAL_ASSISTANT);
  const [mode, setMode] = useState("interactive");
  const [model, setModel] = useState("gpt-5.6-sol");
  const [models, setModels] = useState<string[]>([]);
  const [modelLabels, setModelLabels] = useState<Record<string, string>>({});
  /** The model runs on one local endpoint that every chat shares; see `ready`. */
  const [modelShared, setModelShared] = useState(false);
  /** Whether the default model's provider is set up at all; the composer's "No model
   *  connected" chip reads it. True to begin with, so the chip doesn't flash before settings
   *  arrive; loadSettings corrects it. */
  const [modelReady, setModelReady] = useState(true);
  /** Context windows in tokens for verified models ({full id → tokens}), for the usage chip's meter. */
  const [modelContextWindows, setModelContextWindows] = useState<Record<string, number>>({});
  const [surfaces, setSurfaces] = useState<SurfaceVisibility>({ cowork: true, chat: false, code: false });

  // The conversation on screen.
  const [sessionId, setSessionIdState] = useState<string>(newId());
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [items, setItems] = useState<Item[]>([]);
  const [todo, setTodo] = useState<TodoItem[]>([]);
  const [connected, setConnected] = useState(false);
  const [running, setRunning] = useState(false);
  /** "Compacting context…" (OPE-27): raised by the `compacting` event and dropped by whatever
   *  the engine sends next; otherwise the summary call is a silent pause of several seconds. */
  const [compacting, setCompacting] = useState(false);
  // The streamed answer and the live thinking text (reasoning_delta). Thinking is attached to
  // the assistant item when the message is final, and cleared at turn_start.
  const [streaming, streamingRef, setStreaming] = useMirrored("");
  const [reasoningStream, reasoningRef, setReasoningStream] = useMirrored("");
  /** Token use for this session (OPE-42): rebuilt from the transcript when a session loads,
   *  added to by assistant_message events, and reset with the transcript. */
  const [usage, setUsage] = useState<SessionUsage>(emptyUsage());
  // When the current turn began and what it has used so far; both reset at turn_start so a
  // second turn never carries the first one's clock.
  const [turnStartedAt, setTurnStartedAt] = useState<number | null>(null);
  const [turnTokens, setTurnTokens] = useState(0);

  // Automations. `runContext` says which task an open __run__ session belongs to, for its
  // banner and "Back to runs" (owner, 2026-07-04); without it a run session still shows a
  // generic banner, recognised by its id.
  const [runContext, setRunContext] = useState<{ id: string; title: string } | null>(null);
  /** Which automation the Automations surface opens on, from the banner's Back link or a click
   *  in the sidebar's Scheduled band. Cleared on leaving the surface: a remembered id that went
   *  stale (the automation was deleted) once reopened a dead page stuck on "Loading…" (owner,
   *  2026-07-20). Coming back through the nav should land on the list. */
  const [scheduledOpenId, setScheduledOpenId] = useState<string | null>(null);
  /** Counts presses of the sidebar's Scheduled "+"; ScheduledView explains why it counts. */
  const [scheduledNew, setScheduledNew] = useState(0);

  // Where you are lives in the URL, not in state. `setSurface` keeps its old signature so every
  // caller reads the same; it writes the hash, which is what gives deep links, Back and reload
  // in place.
  const [route, navigate] = useRoute();
  const surface = route.surface;
  const setSurface = useCallback((to: RouteSurface, tab?: string) => navigate({ surface: to, tab }), [navigate]);
  const settingsTab = (route.surface === "settings" ? route.tab : undefined) as SetTab | undefined;
  const openSettings = (tab: SetTab = "appearance") => navigate({ surface: "settings", tab });

  useEffect(() => {
    if (surface !== "scheduled") setScheduledOpenId(null);
  }, [surface]);

  /** Clears what belongs to the conversation being LEFT.
   *
   *  It is its own function because `selectSession` did this and the deep-link effect, whose
   *  comment promised to switch "exactly as clicking the row in the sidebar would", did not:
   *  it took the new id and loaded the transcript, and left the old chat's live turn on screen.
   *  Leaving a running chat by Back, Forward or a pasted link left the next one with that
   *  turn's status line and a composer locked on "Working…", where nothing was running, and
   *  nothing would ever come to correct it. If the session being opened IS running, the server
   *  replays `turn_start` on connect and all of this returns; clearing first is what lets the
   *  two cases be told apart. */
  const leaveConversation = (via: string) => {
    TRACE && trace("chat", "leaving conversation", { via, from: sessionId });
    setTodo([]);
    setStreaming("");
    setReasoningStream("");
    setRunning(false);
    setTurnTokens(0);
  };

  return {
    workspace, setWorkspace, branch, setBranch, showGate, setShowGate, workspaceTrustRequest, setWorkspaceTrustRequest,
    agent, setAgent, model, setModel, modelShared, setModelShared, models, setModels, modelLabels, setModelLabels,
    modelContextWindows, setModelContextWindows, usage, setUsage, surfaces, setSurfaces, mode, setMode, connected,
    setConnected, running, setRunning, compacting, setCompacting, items, setItems, streaming, streamingRef, setStreaming,
    reasoningStream, reasoningRef, setReasoningStream, todo, setTodo, sessions, setSessions, projects, setProjects,
    sessionId, setSessionIdState, runContext, setRunContext, scheduledOpenId, setScheduledOpenId, scheduledNew,
    setScheduledNew, gateCreate, setGateCreate, modelReady, setModelReady, route, navigate, surface, setSurface,
    settingsTab, openSettings, turnStartedAt, setTurnStartedAt, turnTokens, setTurnTokens, leaveConversation,
  };
}
