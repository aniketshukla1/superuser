import { useState, type PointerEvent } from "react";
import { baseName } from "../../paths";
import { isProjectScoped, shortPersonaName } from "../../personaScope";
import { waitingLabel } from "../../streamGate";
import { isTauri, platformOS, startWindowDrag } from "../../tauri";
import type { Item } from "../../types";
import type { useAppState } from "./useAppState";
import type { useBoot } from "./useBoot";
import type { useConversationActions } from "./useConversationActions";
import type { useFollowScroll } from "./useFollowScroll";
import type { useLayout } from "./useLayout";
import type { useNavigationState } from "./useNavigationState";
import type { usePersonas } from "./usePersonas";
import type { useSessionSocket } from "./useSessionSocket";
import type { useSessionSwitching } from "./useSessionSwitching";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useTurnActions } from "./useTurnActions";
import type { useVoiceAndSend } from "./useVoiceAndSend";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> &
  ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> &
  ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend> &
  ReturnType<typeof useTurnActions> & ReturnType<typeof useSessionSwitching> & ReturnType<typeof useConversationActions>;

type Requestable = "approval" | "dirreq" | "planreq" | "question" | "form" | "computer_help" | "computer_secret";

/** The newest request of `kind` that hasn't been answered yet. */
function newestOpen<K extends Requestable>(items: Item[], kind: K): Extract<Item, { kind: K }> | undefined {
  for (let at = items.length - 1; at >= 0; at--) {
    const item = items[at];
    if (item.kind === kind && !(item as { resolved?: unknown }).resolved) return item as Extract<Item, { kind: K }>;
  }
  return undefined;
}

/** What the window's chrome shows: titles, the requests waiting, the panel's tabs, and the
 *  desktop overlay layout. */
export function useChrome(prev: Deps) {
  const { agent, items, model, modelShared, personaOf, railChosen, railHidden, renameConversation, sessionId, sessions, setRailHidden, streaming, workspace } = prev;
  const idle = items.length === 0 && !streaming;

  // Another chat is running this same model on an endpoint that serves one at a time. Both
  // conditions matter: a cloud model takes many chats at once, and "waiting its turn" there
  // would explain a wait that isn't happening. `liveness` comes from the polled session list,
  // which also draws the sidebar's working dots.
  const contended = modelShared && sessions.some((s) => s.session_id !== sessionId && s.liveness === "working" && s.model === model);
  // The waiting row's words, or null while a step runs (the step group shows that).
  const waitLabel = waitingLabel(items, contended);

  /** A tool the topbar asked the panel to open. The counter lets the same button open it again
   *  after you've switched tabs by hand. */
  const [panelRequest, setPanelRequest] = useState<{ tool: "terminal" | "browser"; n: number } | null>(null);
  /** Bumped by the topbar's Artifacts button so the panel goes to Files; a counter for the same
   *  reason as `openAccessKey`, since pressing it again after leaving the tab must bring you back. */
  const [filesRequest, setFilesRequest] = useState(0);
  /** The tab the panel says it is showing. A tool counts as open only when the panel is both
   *  showing and on it: a hidden panel's terminal is not one you can see. */
  const [panelTab, setPanelTab] = useState<string>("activity");
  const toolOpen = (tool: string) => !railHidden && panelTab === tool;
  /** Tools the panel is KEEPING alive (a shell still running, a page still loaded) apart from the
   *  one on screen. Both stay mounted once opened, so only the panel knows; it reports up so the
   *  topbar can mark them. */
  const [liveTools, setLiveTools] = useState<string[]>([]);
  const toolLive = (tool: string) => liveTools.includes(tool);
  const [sessionMenu, setSessionMenu] = useState(false);
  /** Renaming happens in the heading itself. It used to be `window.prompt`, a system dialog for
   *  a one-word edit that WKWebView doesn't always show at all. */
  const [renamingTitle, setRenamingTitle] = useState<string | null>(null);

  // A real toggle: the labels always promised "click to put it away", but it only ever opened,
  // so pressing the lit button visibly did nothing.
  const openTool = (tool: "terminal" | "browser") => {
    railChosen.current = true;
    const close = toolOpen(tool);
    setRailHidden(close);
    if (!close) setPanelRequest((before) => ({ tool, n: (before?.n ?? 0) + 1 }));
  };

  const pendingApproval = newestOpen(items, "approval");
  const pendingDirReq = newestOpen(items, "dirreq");
  const pendingPlan = newestOpen(items, "planreq");
  const pendingQuestion = newestOpen(items, "question");
  const pendingForm = newestOpen(items, "form");
  const pendingHelp = newestOpen(items, "computer_help");
  const pendingSecret = newestOpen(items, "computer_secret");
  const nameOf = (persona: string) => shortPersonaName(personaOf(persona)?.name, persona) || "The assistant";

  // The heading names WHO you are talking to and WHERE, not the model behind it (owner's call,
  // replacing 2026-07-22's "drop the persona name", which was made while assistants were
  // hidden). "Coder · aegis" says something the sidebar's highlighted row stops saying once it
  // has scrolled away. The model lives on the composer's chip, where it can also be changed;
  // up here it was a fact you couldn't act on. The facts line (§22) only appears once there is
  // history: before that the model can still be chosen, so there is nothing fixed to state.
  const hasHistory = items.length > 0;
  const personaLabel = shortPersonaName(personaOf(agent)?.name, agent);
  const facts = personaLabel ? [personaLabel] : [];
  if (workspace && isProjectScoped(personaOf(agent))) facts.push(baseName(workspace));
  const activeInfo = sessions.find((s) => s.session_id === sessionId);
  // A title guessed from the prompt suits a LIST (a column of "New session" can't be scanned)
  // but makes a poor heading: your own question echoed back in large type tells you nothing,
  // and a long one fills the top of the page with your words. So the heading is a name you
  // CHOSE, or else the assistant.
  const activeTitle = activeInfo?.named ? activeInfo.title : personaLabel || (activeInfo ? "Chat" : "New session");
  // Only what the heading isn't already saying.
  const subtitleShown = activeInfo?.named ? facts : facts.filter((fact) => fact !== personaLabel);
  const commitTitle = () => {
    const typed = renamingTitle?.trim();
    setRenamingTitle(null);
    if (typed && activeInfo && typed !== activeInfo.title) renameConversation(sessionId, typed);
  };

  const desktop = isTauri();
  // In development, `?overlay=1` imitates the desktop overlay layout in a browser (the
  // tauri-overlay class and stand-in window buttons where the real ones sit), so the corner can
  // be tuned in the preview without building the app. isTauri() keeps it out of the real app.
  const simOverlay = !desktop && new URLSearchParams(window.location.search).has("overlay");
  // The overlay layout is for macOS ONLY. Windows and Linux keep the native title bar, and the
  // Mac's adjustments (window-button insets, lowered top strips) looked misaligned under it
  // (caught 2026-07-21).
  const overlay = simOverlay || (desktop && platformOS() === "macos");
  const beginWindowDrag = (event: PointerEvent) => {
    if (desktop && event.button === 0) startWindowDrag();
  };

  return { idle, waitLabel, panelRequest, filesRequest, setFilesRequest, setPanelTab, toolOpen, setLiveTools, toolLive, sessionMenu, setSessionMenu, renamingTitle, setRenamingTitle, openTool, pendingApproval, pendingDirReq, pendingPlan, pendingQuestion, pendingForm, pendingHelp, pendingSecret, nameOf, hasHistory, activeInfo, activeTitle, subtitleShown, commitTitle, simOverlay, overlay, beginWindowDrag };
}
