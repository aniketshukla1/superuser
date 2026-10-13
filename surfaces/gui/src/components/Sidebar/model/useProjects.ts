import { useState, useEffect } from "react";
import type { AppSurface } from "../../../nav";
import type { Props } from "../Props";
import type { useAccountMenu } from "./useAccountMenu";
import type { useAutomationsList } from "./useAutomationsList";
import type { useRowMenu } from "./useRowMenu";
import type { usePersonaLayout } from "./usePersonaLayout";

/** Which folder is open, show-all sets, pinned sessions, and where each nav entry goes. */
export function useProjects(prev: { props: Props } & ReturnType<typeof useAccountMenu> & ReturnType<typeof useAutomationsList> & ReturnType<typeof useRowMenu> & ReturnType<typeof usePersonaLayout>) {
  const { props } = prev;
 // the persona whose sessions the body shows

  // Folders open the way persona groups do, because they are the same gesture on the
  // same kind of thing: ONE open at a time, seeded to the one you are in, following it
  // when it changes. It used to be a Set XOR'd against a per-folder `defaultOpen`, which
  // gave the two tabs visibly different behaviour — several folders could stand open at
  // once, and because `defaultOpen` moved when the workspace moved, a folder you had
  // shut could reopen itself and one you had opened could close, with no click involved.
  //
  // `undefined` is not `null`: undefined means you have not chosen yet, so the top
  // folder stands in and the list never opens shut. Clicking always writes a concrete
  // answer — a folder, or null for "all closed".
  const [openProject, setOpenProject] = useState<string | null | undefined>(
    props.workspace || undefined,
  );
  useEffect(() => setOpenProject(props.workspace || undefined), [props.workspace]);
  /** Which folder is showing as open, given the folders actually on screen. Three
   *  states collapse to one answer here: never chosen (the top folder stands in),
   *  deliberately closed (nothing), or chosen — unless a search has since hidden the
   *  chosen one, in which case the top surviving folder takes over rather than
   *  leaving a list of closed folders with the answer inside one of them. */
  const resolveOpenProject = (order: string[]): string | null => {
    if (openProject === undefined) return order[0] ?? null;
    if (openProject === null) return null;
    return order.includes(openProject) ? openProject : (order[0] ?? null);
  };
  /** The same gesture as onHeaderClick: the open one closes, anything else becomes the
   *  only one open. The caller passes what is showing, because `undefined` resolves
   *  against a different list in each of the two places folders are drawn. */
  const toggleProject = (proj: string, showing: string | null) =>
    setOpenProject(showing === proj ? null : proj);
  // `projShowAll` lifts the peek cap for a given folder; `personaShowAll` does the same
  // for a (non-project) persona's flat session list.

  const [projShowAll, setProjShowAll] = useState<Set<string>>(new Set());
  const [personaShowAll, setPersonaShowAll] = useState<Set<string>>(new Set());
  const toggleSet = (set: Set<string>, key: string) => {
    const next = new Set(set);
    next.has(key) ? next.delete(key) : next.add(key);
    return next;
  };

  // Pinned sessions across ALL personas — the cross-persona band at the top (manual pins only).
  const pinnedSessions = props.sessions.filter(
    (s) => s.pinned && !s.session_id.startsWith("__") && !s.archived,
  );
  // §31 (revised 2026-07-21): mention-spawned sessions list chronologically in Recent like any
  // other session — the OriginIcon in the row's indicator cluster marks where they came from.
  // The separate collapsed "From Slack" band hid fresh mentions below week-old sessions.
  // The one place a nav entry's id becomes behavior. `nav.ts` owns label/icon/order;
  // this owns where a click goes and whether the entry reads as current. Both the
  // rail rows and the account-menu items resolve through here, so the two lists
  // cannot disagree about what "Inbox" means or which surface is open.
  const navTarget = (id: AppSurface): { go: () => void; active: boolean } => {
    switch (id) {
      case "brain":
        return { go: props.onOpenBrain, active: props.brainActive };
      case "scheduled":
        return { go: props.onOpenScheduled, active: props.scheduledActive };
      case "inbox":
        return { go: props.onOpenInbox, active: props.inboxActive };
      case "integrations":
        return { go: props.onOpenIntegrations, active: props.integrationsActive };
      case "audit":
        return { go: props.onOpenAudit, active: props.auditActive };
    }
  };

  return { resolveOpenProject, toggleProject, projShowAll, setProjShowAll, personaShowAll, setPersonaShowAll, toggleSet, pinnedSessions, navTarget };
}
