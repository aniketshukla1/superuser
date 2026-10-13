import { useMemo, useEffect } from "react";
import type { SessionInfo } from "../../../types";
import type { Persona, SurfaceVisibility } from "../../../api";
import { surfaceFromPersona, SURFACES } from "../Props";
import type { Props } from "../Props";
import type { useAccountMenu } from "./useAccountMenu";
import type { useAutomationsList } from "./useAutomationsList";
import type { useRowMenu } from "./useRowMenu";
import type { usePersonaLayout } from "./usePersonaLayout";
import type { useProjects } from "./useProjects";
import type { useSessionLists } from "./useSessionLists";

/** Sessions by project and by kind (chat vs code), and which surfaces show. */
export function useProjectLists(prev: { props: Props } & ReturnType<typeof useAccountMenu> & ReturnType<typeof useAutomationsList> & ReturnType<typeof useRowMenu> & ReturnType<typeof usePersonaLayout> & ReturnType<typeof useProjects> & ReturnType<typeof useSessionLists>) {
  const { mine, matches, normalizedQuery, browseKey, personas, personaVisible, surfaceTab,
    setSurfaceTab, props } = prev;
  // Code/Cowork group by project; Chat is a flat recents list.
  const byProject = useMemo(() => {
    const grouped = new Map<string, SessionInfo[]>();
    for (const s of mine) {
      if (!grouped.has(s.workspace)) grouped.set(s.workspace, []);
      grouped.get(s.workspace)!.push(s);
    }
    return grouped;
  }, [mine]);

  const filteredByProject = useMemo(() => {
    const grouped = new Map<string, SessionInfo[]>();
    for (const [proj, list] of byProject) grouped.set(proj, list.filter(matches));
    return grouped;
  }, [byProject, normalizedQuery]);

  // Projects are tracked PER SURFACE: a folder appears under Code only if it has Code sessions,
  // under Cowork only if it has Cowork sessions (+ the currently-open folder). No cross-bleed.
  const projectOrder: string[] = [];
  const seen = new Set<string>();
  // Pin the active folder at top only when browsing the active persona (else it belongs elsewhere).
  if (props.workspace && browseKey === props.agent) {
    projectOrder.push(props.workspace);
    seen.add(props.workspace);
  }
  for (const s of mine) {
    if (s.workspace && !seen.has(s.workspace)) {
      seen.add(s.workspace);
      projectOrder.push(s.workspace);
    }
  }

  // Surfaced + enabled personas drive the surface list (default persona first); fall back to the
  // static set until loaded. A persona that has live sessions ALWAYS gets a section, surfaced or
  // not — every session must have a home in the grouped layout (a picker preference can hide the
  // persona from New Session, never orphan its conversations).
  const agentsWithSessions = new Set([
    // The conversation on screen right now, which is live by definition. A session
    // reaches `props.sessions` only once it has a turn, so a brand-new one was in
    // no list at all — and if its assistant is hidden from the picker, the sidebar
    // had nowhere to put it. You clicked an assistant, the chat area switched, and
    // the left nav showed no sign of it. The rule stated just above says a session
    // must never be orphaned; an empty one is still a session.
    props.agent,
    ...props.sessions
      .filter((s) => !s.archived && !s.session_id.startsWith("__"))
      .map((s) => s.agent),
  ]);
  /// Does this assistant get a section in the sidebar? Either you chose to see it,
  /// or it is holding a conversation. Written once because it was written twice and
  /// the two copies are what let them drift.
  const hasHome = (p: Persona) => (p.enabled && p.surfaced) || agentsWithSessions.has(p.id);
  const visibleSurfaces = (
    personas
      ? personas
          .filter(hasHome)
          .sort((a, b) => Number(b.default) - Number(a.default)) // default leads
          .map(surfaceFromPersona)
      : SURFACES.filter(
          (s) => s.key === "cowork" || props.surfaces[s.key as keyof SurfaceVisibility],
        )
  )
    .filter((s) => personaVisible(s.key))
    // Assistants first, workbenches last. The split is CAPABILITY (a workbench runs
    // shell and is bound to a place) rather than provenance — presets we ship later
    // are still assistants, and a shell-using persona a user writes is a workbench.
    .sort((a, b) => Number((a as any).kind === "workbench") - Number((b as any).kind === "workbench"));

  const isWorkbench = (s: { kind?: string }) => s.kind === "workbench";
  // Only offer the choice when there is one. Someone with no workbenches would
  // otherwise get a Code tab that leads to an empty list — the same reasoning the
  // group headings below already use to decide whether to label anything.
  const hasBothKinds =
    visibleSurfaces.some((s) => isWorkbench(s as any)) &&
    visibleSurfaces.some((s) => !isWorkbench(s as any));
  const shownSurfaces = hasBothKinds
    ? visibleSurfaces.filter((s) => isWorkbench(s as any) === (surfaceTab === "code"))
    : visibleSurfaces;

  // Follow the active session into its own half. Without this, opening a Coder
  // session while the control said Chat would hide the persona you are currently
  // talking to — the sidebar showing one thing and the chat area another, which is
  // exactly the bug the New-session button had.
  const activeIsWorkbench = visibleSurfaces.some(
    (s) => s.key === props.agent && isWorkbench(s as any),
  );
  useEffect(() => {
    setSurfaceTab(activeIsWorkbench ? "code" : "chat");
  }, [activeIsWorkbench, props.agent]);

  // ---- project-first grouping (the Code tab) ----
  //
  // Under Code the folder is the thing you navigate by, not the persona. Grouping
  // by persona first and folder second meant the same project appeared under Coder
  // AND under Ops, and you had to remember which assistant you happened to use last
  // to find the work. Projects are top level here and personas fall away — the
  // session row already carries which one it belongs to when it matters.
  //
  // Chat keeps the persona list: those sessions have no folder to group by.
  const workbenchKeys = useMemo(
    () => new Set(visibleSurfaces.filter((s) => isWorkbench(s as any)).map((s) => s.key)),
    [visibleSurfaces],
  );
  const codeSessions = useMemo(
    () =>
      props.sessions.filter(
        (s) =>
          !s.archived &&
          !s.session_id.startsWith("__") &&
          workbenchKeys.has(s.agent) &&
          matches(s),
      ),
    [props.sessions, workbenchKeys, normalizedQuery],
  );
  const codeByProject = useMemo(() => {
    const g = new Map<string, SessionInfo[]>();
    for (const s of codeSessions) {
      // A workbench session with no folder yet is still work — it gets its own
      // group rather than vanishing, keyed on "" and labelled below.
      const k = s.workspace || "";
      if (!g.has(k)) g.set(k, []);
      g.get(k)!.push(s);
    }
    return g;
  }, [codeSessions]);
  // Active folder first, then by most recent session — the order you would look in.
  const codeProjectOrder = useMemo(() => {
    const keys = [...codeByProject.keys()];
    const recency = (k: string) =>
      Math.max(...(codeByProject.get(k) || []).map((s) => Date.parse(s.updated_at || "") || 0));
    keys.sort((a, b) => {
      if (a === props.workspace) return -1;
      if (b === props.workspace) return 1;
      // The folderless group sinks to the bottom: it is a holding pen, not a project.
      if (a === "") return 1;
      if (b === "") return -1;
      return recency(b) - recency(a);
    });
    return keys;
  }, [codeByProject, props.workspace]);

  /** The Code tab's list: folders at the top level, sessions inside them. */
  // The persona a project's `+` starts a session as. Staying on the workbench you are
  // already in beats guessing; when you are not on one (you are in Chat, or on a
  // non-workbench persona), the folder's own most recent session is the answer.
  const projectAgent = (list: SessionInfo[]): string =>
    workbenchKeys.has(props.agent) ? props.agent : list[0]?.agent || browseKey;

  return { filteredByProject, projectOrder, hasHome, hasBothKinds, shownSurfaces, codeByProject, codeProjectOrder, projectAgent };
}
