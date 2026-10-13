import { useState, useRef, useCallback, useEffect } from "react";
import type { WorkspaceCommandTrust, SurfaceVisibility, RecentWorkspace } from "../../api";
import { GENERAL_ASSISTANT, newId } from "../appHelpers";
import type { SessionUsage, Item, TodoItem, SessionInfo } from "../../types";
import { emptyUsage } from "../../usage";
import { useRoute, type RouteSurface } from "../../route";
import type { SetTab } from "../../components/SettingsView";
import { TRACE, trace } from "../../trace";

/** The app's core state: workspace, assistant, model, transcript, sessions and route. */
export function useAppState() {
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [branch, setBranch] = useState<string | null>(null);
  const [showGate, setShowGate] = useState(false);
  const [workspaceTrustRequest, setWorkspaceTrustRequest] =
    useState<WorkspaceCommandTrust | null>(null);
  const [agent, setAgent] = useState(GENERAL_ASSISTANT);
  const [model, setModel] = useState("gpt-5.6-sol");
  /** This model runs on one local endpoint shared by every chat — see `ready`. */
  const [modelShared, setModelShared] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const [modelLabels, setModelLabels] = useState<Record<string, string>>({});
  // {full model id → context window in tokens} from the curated matrix (verified only);
  // drives the composer usage chip's context-fill meter.
  const [modelContextWindows, setModelContextWindows] = useState<Record<string, number>>({});
  // Per-session token usage (OPE-42): rebuilt from the transcript on session load,
  // accumulated live from assistant_message events, reset with the transcript.
  const [usage, setUsage] = useState<SessionUsage>(emptyUsage());
  const [surfaces, setSurfaces] = useState<SurfaceVisibility>({ cowork: true, chat: false, code: false });
  const [mode, setMode] = useState("interactive");
  const [connected, setConnected] = useState(false);
  const [running, setRunning] = useState(false);
  // Transient "Compacting context…" indicator (OPE-27): set by the `compacting` event,
  // cleared by whatever the engine emits next — the summarizer call is otherwise a
  // multi-second silent stall mid-turn.
  const [compacting, setCompacting] = useState(false);
  const [items, setItems] = useState<Item[]>([]);
  const [streaming, setStreamingState] = useState("");
  // Ref mirror of `streaming`: the WS handler closure is built once per socket and can't read
  // fresh state — the interrupted/error flush below needs the live buffer at event time.
  const streamingRef = useRef("");
  const setStreaming = (value: string | ((s: string) => string)) => {
    streamingRef.current = typeof value === "function" ? value(streamingRef.current) : value;
    setStreamingState(streamingRef.current);
  };
  // The turn's live thinking text (reasoning_delta events) — same ref-mirror pattern.
  // Folded onto the assistant item when the message finalizes; cleared on turn_start.
  const [reasoningStream, setReasoningStreamState] = useState("");
  const reasoningRef = useRef("");
  const setReasoningStream = (value: string) => {
    reasoningRef.current = value;
    setReasoningStreamState(value);
  };
  const [todo, setTodo] = useState<TodoItem[]>([]);
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [projects, setProjects] = useState<RecentWorkspace[]>([]);
  const [sessionId, setSessionIdState] = useState<string>(newId());
  // Automation-run context (§ owner ask 2026-07-04): which task an open __run__ session belongs
  // to, driving the banner + "Back to runs". Best-effort — a run session without context still
  // shows a generic banner (detected by its __run__ id).
  const [runContext, setRunContext] = useState<{ id: string; title: string } | null>(null);
  // Which automation the Automations surface opens on (set by the banner's Back link
  // or a sidebar Scheduled-band click). Cleared on leaving the surface: a remembered
  // id going stale (e.g. the automation was deleted) reopened a dead detail —
  // "Loading…" forever (owner-hit 2026-07-20). Nav re-entry should land on the list.
  const [scheduledOpenId, setScheduledOpenId] = useState<string | null>(null);
  // Bumped by the sidebar's Scheduled "+". See ScheduledView for why it counts
  // rather than flags.
  const [scheduledNew, setScheduledNew] = useState(0);
  const [gateCreate, setGateCreate] = useState(false);
  // Whether the default model's provider is actually configured (any provider). Drives the
  // composer's "No model connected" chip. Default true so we don't flash the chip before settings
  // load; corrected by loadSettings.
  const [modelReady, setModelReady] = useState(true);

  // Where we are lives in the URL now, not in state. `setSurface` keeps its old
  // shape so every call site reads the same — it just writes the hash instead of
  // a useState, which is what buys deep links, Back, and reload-in-place.
  const [route, navigate] = useRoute();
  const surface = route.surface;
  const setSurface = useCallback(
    (s: RouteSurface, tab?: string) => navigate({ surface: s, tab }),
    [navigate],
  );
  const settingsTab = (route.surface === "settings" ? route.tab : undefined) as SetTab | undefined;
  const openSettings = (tab: SetTab = "appearance") => navigate({ surface: "settings", tab });
  // A remembered Scheduled-detail target must not outlive the surface (see the
  // scheduledOpenId comment above): nav re-entry lands on the list, never a
  // possibly-deleted automation's dead detail.
  useEffect(() => {
    if (surface !== "scheduled") setScheduledOpenId(null);
  }, [surface]);
  // When the current turn started, and what it has cost so far. Both reset on turn_start so
  // a second turn never inherits the first one's clock.
  const [turnStartedAt, setTurnStartedAt] = useState<number | null>(null);
  const [turnTokens, setTurnTokens] = useState(0);
  /** Everything that belongs to the conversation you are LEAVING.
   *
   * Extracted because it was written out in `selectSession` and not in the deep-link
   * effect below, which claimed in a comment to switch "exactly as clicking the row
   * in the sidebar would". It did not: it adopted the id and loaded the transcript,
   * and left the previous chat's live turn on screen. Walking away from a running
   * chat — Back, Forward, a pasted link — left the next one showing that turn's
   * status line and its composer locked on "Working…", in a conversation where
   * nothing was running. It never settled, because no event was coming to correct it.
   *
   * If the session being opened IS running, the server replays `turn_start` on
   * connect and these come straight back. Clearing first is what makes the two cases
   * distinguishable at all. */
  const leaveConversation = (via: string) => {
    TRACE && trace("chat", "leaving conversation", { via, from: sessionId });
    setTodo([]);
    setStreaming("");
    setReasoningStream("");
    setRunning(false);
    setTurnTokens(0);
  };

  return { workspace, setWorkspace, branch, setBranch, showGate, setShowGate, workspaceTrustRequest, setWorkspaceTrustRequest, agent, setAgent, model, setModel, modelShared, setModelShared, models, setModels, modelLabels, setModelLabels, modelContextWindows, setModelContextWindows, usage, setUsage, surfaces, setSurfaces, mode, setMode, connected, setConnected, running, setRunning, compacting, setCompacting, items, setItems, streaming, streamingRef, setStreaming, reasoningStream, reasoningRef, setReasoningStream, todo, setTodo, sessions, setSessions, projects, setProjects, sessionId, setSessionIdState, runContext, setRunContext, scheduledOpenId, setScheduledOpenId, scheduledNew, setScheduledNew, gateCreate, setGateCreate, modelReady, setModelReady, route, navigate, surface, setSurface, settingsTab, openSettings, turnStartedAt, setTurnStartedAt, turnTokens, setTurnTokens, leaveConversation };
}
