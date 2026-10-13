import { useState, useEffect, useRef } from "react";
import { type Persona, getPersonas, PERSONAS_CHANGED, getSettings, setNavLayout } from "../../../api";
import { showPersonas } from "../../../flags";
import { useNavVisible } from "../../../navPrefs";
import type { Props } from "../Props";
import type { useAccountMenu } from "./useAccountMenu";
import type { useAutomationsList } from "./useAutomationsList";
import type { useRowMenu } from "./useRowMenu";

/** Personas, grouped/flat layout, filters, the More and Scheduled disclosures, and which persona is open. */
export function usePersonaLayout(prev: { props: Props } & ReturnType<typeof useAccountMenu> & ReturnType<typeof useAutomationsList> & ReturnType<typeof useRowMenu>) {
  const { props } = prev;
  const [showArchived, setShowArchived] = useState(false);
  // Surfaced + enabled personas drive the surface list + family-aware behavior.
  // Refetched on the personas-changed event so an enable/install/delete in Settings
  // shows up here immediately (no page refresh).
  const [personas, setPersonas] = useState<Persona[] | null>(null);
  useEffect(() => {
    const load = () =>
      getPersonas()
        .then(setPersonas)
        .catch(() => setPersonas(null));
    load();
    window.addEventListener(PERSONAS_CHANGED, load);
    return () => window.removeEventListener(PERSONAS_CHANGED, load);
  }, []);
  const personaOf = (id: string) => personas?.find((p) => p.id === id);

  // Sidebar layout (§7): "grouped" = the per-persona accordion; "flat" = a single ungrouped list
  // (Pinned + Recent). Read the persisted preference on load; ABSENT falls back by the
  // Personas flag — with personas hidden for launch, a per-persona accordion groups by
  // a concept the user can't see, so the default is the flat chronological list
  // (owner call 2026-07-20). An explicit stored choice always wins.
  const defaultLayout: "flat" | "grouped" = showPersonas() ? "grouped" : "flat";
  const [layout, setLayout] = useState<"flat" | "grouped">(defaultLayout);
  // Sessions shown per group before "Show more" — Settings ▸ Appearance ▸ Sidebar.
  const [peek, setPeek] = useState(5);
  useEffect(() => {
    getSettings()
      .then((s) => {
        setLayout(
          s.nav_layout === "flat" ? "flat" : s.nav_layout === "grouped" ? "grouped" : defaultLayout,
        );
        if (s.sessions_peek) setPeek(s.sessions_peek);
      })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const setGroupBy = (next: "flat" | "grouped") => {
    setLayout(next);
    setNavLayout(next).catch(() => {});
  };
  // Chronological RECENT list: cap at RECENT_PEEK with a Show more/less toggle so the sidebar
  // doesn't grow unbounded.
  const RECENT_PEEK = 4;
  const [recentExpanded, setRecentExpanded] = useState(false);
  // The RECENT-header group/filter popover (§20). Filter = show only these personas (empty = all).
  const [groupMenuOpen, setGroupMenuOpen] = useState(false);
  const [filterPersonas, setFilterPersonas] = useState<Set<string>>(new Set());
  const toggleFilterPersona = (id: string) =>
    setFilterPersonas((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  const personaVisible = (agent: string) =>
    filterPersonas.size === 0 || filterPersonas.has(agent);

  // Which accordion body is expanded. Decoupled from the active session (props.agent): expanding
  // a persona BROWSES its sessions without switching the chat area. Selecting a session or "New
  // session" is what switches (and re-opens that persona). Falls back to the active persona.
  // The More disclosure. Collapsed by default — the whole point is that the rail
  // is two rows until you ask for the rest.
  // Scheduled collapses like an assistant group does. Open by default: it holds a
  // handful of rows and hiding them by default would be hiding the feature.
  const [scheduledOpen, setScheduledOpen] = useState(true);
  const [moreOpen, setMoreOpen] = useState(false);
  const moreButtonRef = useRef<HTMLButtonElement | null>(null);
  const moreMenuRef = useRef<HTMLDivElement | null>(null);
  // Focus moves into the menu when it opens, so arrow keys and Escape work at once.
  useEffect(() => {
    if (moreOpen) moreMenuRef.current?.querySelector<HTMLElement>("[role=menuitem]")?.focus();
  }, [moreOpen]);
  const [editSidebarOpen, setEditSidebarOpen] = useState(false);
  const [navVisible, setNavVisible] = useNavVisible();
  // Which half of the persona list the sidebar is showing. The split is the one
  // the list already made: an assistant talks, a workbench runs a shell and is
  // bound to a folder. Stacking both meant scrolling past the kind you weren't
  // using to reach the one you were.
  const [surfaceTab, setSurfaceTab] = useState<"chat" | "code">("chat");
  const [openKey, setOpenKey] = useState<string | null>(props.agent);
  useEffect(() => setOpenKey(props.agent), [props.agent]);
  const browseKey = openKey ?? props.agent;

  return { showArchived, setShowArchived, personas, personaOf, layout, peek, setGroupBy, RECENT_PEEK, recentExpanded, setRecentExpanded, groupMenuOpen, setGroupMenuOpen, filterPersonas, setFilterPersonas, toggleFilterPersona, personaVisible, scheduledOpen, setScheduledOpen, moreOpen, setMoreOpen, moreButtonRef, moreMenuRef, editSidebarOpen, setEditSidebarOpen, navVisible, setNavVisible, surfaceTab, setSurfaceTab, openKey, setOpenKey, browseKey };
}
