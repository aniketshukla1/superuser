import { waitingLabel } from "../../streamGate";
import { useState, type PointerEvent } from "react";
import { shortPersonaName, isProjectScoped } from "../../personaScope";
import { baseName } from "../../paths";
import { isTauri, platformOS, startWindowDrag } from "../../tauri";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";
import type { useLayout } from "./useLayout";
import type { usePersonas } from "./usePersonas";
import type { useBoot } from "./useBoot";
import type { useSplashAndSettings } from "./useSplashAndSettings";
import type { useSessionSocket } from "./useSessionSocket";
import type { useFollowScroll } from "./useFollowScroll";
import type { useVoiceAndSend } from "./useVoiceAndSend";
import type { useTurnActions } from "./useTurnActions";
import type { useSessionSwitching } from "./useSessionSwitching";
import type { useConversationActions } from "./useConversationActions";

/** What the window chrome shows: titles, pending requests, panel tabs and overlay layout. */
export function useChrome(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState> & ReturnType<typeof useLayout> & ReturnType<typeof usePersonas> & ReturnType<typeof useBoot> & ReturnType<typeof useSplashAndSettings> & ReturnType<typeof useSessionSocket> & ReturnType<typeof useFollowScroll> & ReturnType<typeof useVoiceAndSend> & ReturnType<typeof useTurnActions> & ReturnType<typeof useSessionSwitching> & ReturnType<typeof useConversationActions>) {
  const { items, streaming, modelShared, sessions, sessionId, model, railHidden, railChosen,
      setRailHidden, personaOf, agent, workspace, renameConversation } = prev;
  const idle = items.length === 0 && !streaming;
  // What the waiting row says, or null while a step is in flight (the step group has it).
  // Another chat is running the same model, on an endpoint that serves one at a time.
  // Both halves matter: a cloud model handles concurrent chats, so saying "waiting its
  // turn" there would explain a wait that is not happening. `liveness` comes from the
  // polled session list, which is how the sidebar draws its working dots.
  const modelContended =
    modelShared &&
    sessions.some(
      (s) => s.session_id !== sessionId && s.liveness === "working" && s.model === model,
    );
  const waitLabel = waitingLabel(items, modelContended);
  // A tool the topbar has asked the panel to open. The nonce makes pressing the same button
  // twice re-open it after you have switched tabs by hand.
  const [panelRequest, setPanelRequest] = useState<{ tool: "terminal" | "browser"; n: number } | null>(null);
  // Bumped by the topbar's Artifacts button so the panel lands on Files. Same shape
  // as `openAccessKey`: a counter, because pressing it again after switching tabs
  // away has to bring you back.
  const [filesRequest, setFilesRequest] = useState(0);
  // What the side panel is actually showing, reported by the panel itself. A tool is
  // "on" only when the panel is both open AND on it — a hidden rail showing the
  // terminal is not a terminal you can see.
  const [panelTab, setPanelTab] = useState<string>("activity");
  const toolOpen = (t: string) => !railHidden && panelTab === t;
  // Tools the panel is HOLDING — a shell still running, a page still loaded — as
  // opposed to the one it is showing. Both panes stay mounted once opened, so this
  // is a fact only the panel has; it reports it up so the topbar can mark them.
  const [liveTools, setLiveTools] = useState<string[]>([]);
  const toolLive = (t: string) => liveTools.includes(t);
  const [sessionMenu, setSessionMenu] = useState(false);
  // Renaming happens on the heading itself. It used to be `window.prompt`, which is a system
  // dialog for a one-word edit — and WKWebView does not always show one at all.
  const [renamingTitle, setRenamingTitle] = useState<string | null>(null);
  // A real toggle. The labels have promised "click to put it away" all along while
  // this only ever opened, so pressing the lit button did nothing you could see.
  const openTool = (tool: "terminal" | "browser") => {
    railChosen.current = true;
    if (toolOpen(tool)) {
      setRailHidden(true);
      return;
    }
    setRailHidden(false);
    setPanelRequest((p) => ({ tool, n: (p?.n ?? 0) + 1 }));
  };
  const pendingApproval = [...items].reverse().find((i) => i.kind === "approval" && !i.resolved);
  const pendingDirReq = [...items].reverse().find((i) => i.kind === "dirreq" && !i.resolved);
  const pendingPlan = [...items].reverse().find((i) => i.kind === "planreq" && !i.resolved);
  const pendingQuestion = [...items].reverse().find((i) => i.kind === "question" && !i.resolved);
  const pendingForm = [...items].reverse().find((i) => i.kind === "form" && !i.resolved);
  const pendingHelp = [...items].reverse().find((i) => i.kind === "computer_help" && !i.resolved);
  const pendingSecret = [...items].reverse().find((i) => i.kind === "computer_secret" && !i.resolved);
  const nameOf = (persona: string) => shortPersonaName(personaOf(persona)?.name, persona) || "The assistant";
  // Facts subtitle (§22): the session's FIXED facts, not controls — model (+ the
  // workspace folder for project-scoped sessions). Renders only once the session has history;
  // until then the model is still choosable in the composer, so there's no locked fact to state.
  const hasHistory = items.length > 0;
  // WHO you are talking to and WHERE, not what model is behind it (owner ask, supersedes
  // 2026-07-22's "drop the persona name"). That call was made while personas were hidden, so
  // "Assistant" really was noise; the sidebar now names them, and the header saying "Coder ·
  // aegis" tells you something the sidebar's highlighted row does not once you have scrolled
  // it away.
  //
  // The model does not disappear — it is on the composer's model chip, which is also where
  // you change it. Up here it was a fixed fact you cannot act on, next to a name you can.
  const personaLabel = shortPersonaName(personaOf(agent)?.name, agent);
  const subtitleParts = personaLabel ? [personaLabel] : [];
  if (isProjectScoped(personaOf(agent)) && workspace) subtitleParts.push(baseName(workspace));
  const activeInfo = sessions.find((s) => s.session_id === sessionId);
  // The chat's own heading. A prompt-derived title is a good guess for a LIST — a
  // column of "New session" is unscannable — and a poor heading: showing a fragment
  // of your own question back at you, at heading size, says nothing you don't
  // already know, and a long one turns the top of the page into a wall of your own
  // words. So the heading is the name you CHOSE, or the assistant you're talking to.
  const activeTitle = activeInfo?.named ? activeInfo.title : personaLabel || (activeInfo ? "Chat" : "New session");
  // Whatever the heading is not already saying.
  const subtitleShown = activeInfo?.named ? subtitleParts : subtitleParts.filter((p) => p !== personaLabel);
  const commitTitle = () => {
    const next = renamingTitle?.trim();
    setRenamingTitle(null);
    if (next && activeInfo && next !== activeInfo.title) renameConversation(sessionId, next);
  };

  const desktop = isTauri();
  // Dev-only: `?overlay=1` simulates the desktop overlay layout in the browser (adds the
  // tauri-overlay class + draws fake traffic lights at the real position) so the top-left can be
  // tuned in the preview without a DMG build. Never active in the real app (isTauri() short-circuits).
  const simOverlay = !desktop && new URLSearchParams(window.location.search).has("overlay");
  // Overlay layout is macOS-ONLY: Windows/Linux keep the native title bar, so the mac
  // compensations (traffic-light insets, lowered top strips) must not apply there —
  // they rendered as misalignments under Windows' native bar (caught 2026-07-21).
  const overlay = (desktop && platformOS() === "macos") || simOverlay;
  const beginWindowDrag = (event: PointerEvent) => {
    if (!desktop || event.button !== 0) return;
    startWindowDrag();
  };

  return { idle, waitLabel, panelRequest, filesRequest, setFilesRequest, setPanelTab, toolOpen, setLiveTools, toolLive, sessionMenu, setSessionMenu, renamingTitle, setRenamingTitle, openTool, pendingApproval, pendingDirReq, pendingPlan, pendingQuestion, pendingForm, pendingHelp, pendingSecret, nameOf, hasHistory, activeInfo, activeTitle, subtitleShown, commitTitle, simOverlay, overlay, beginWindowDrag };
}
