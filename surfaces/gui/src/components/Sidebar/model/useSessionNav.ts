import type { Props } from "../Props";
import type { useAccountMenu } from "./useAccountMenu";
import type { useAutomationsList } from "./useAutomationsList";
import type { useRowMenu } from "./useRowMenu";
import type { usePersonaLayout } from "./usePersonaLayout";
import type { useProjects } from "./useProjects";
import type { useSessionLists } from "./useSessionLists";
import type { useProjectLists } from "./useProjectLists";

/** What is current, what is expanded, and opening a session. */
export function useSessionNav(prev: { props: Props } & ReturnType<typeof useAccountMenu> & ReturnType<typeof useAutomationsList> & ReturnType<typeof useRowMenu> & ReturnType<typeof usePersonaLayout> & ReturnType<typeof useProjects> & ReturnType<typeof useSessionLists> & ReturnType<typeof useProjectLists>) {
  const { openKey, setOpenKey, props } = prev;
  const isCurrent = (key: string) => props.agent === key; // the active session's persona
  // The conversation open in the chat area, when it is one the list would show.
  const openSession = props.sessions.find(
    (s) => s.session_id === props.activeSession && !s.session_id.startsWith("__") && !s.archived,
  );
  const isExpanded = (key: string) => openKey === key; // its body is open
  // Expand ≠ switch: clicking a header only browses (toggles the accordion). The chat area
  // changes only when a session is selected or "New session" is clicked.
  const onHeaderClick = (key: string) => setOpenKey((k) => (k === key ? null : key));

  return { isCurrent, openSession, isExpanded, onHeaderClick };
}
