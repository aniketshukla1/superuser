import { useCallback, useEffect, useState } from "react";
import { CHAT_MIN_W, fitBeside, usePanelWidth, useWindowWidth } from "../../components/ResizeHandle";
import { NAV_COLLAPSED_KEY, NAV_DEFAULT_W, PANEL_DEFAULT_W, PANEL_MIN_W } from "../appHelpers";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";

type Deps = ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState>;

/** The narrowest the nav may be. Not a matter of taste: the top bar has a floor. After the
 *  84px the window buttons need, its five controls and their gaps take 129px, plus 4px of
 *  padding on the right, so 217px before anything is clipped. At the old 200 the Workspace
 *  icon fell off the edge, which is how it was reported; 224 leaves the 6px of slack that
 *  `e2e-app/window/` checks for. */
const NAV_MIN_W = 224;

const storedCollapsed = () => {
  try {
    return localStorage.getItem(NAV_COLLAPSED_KEY) === "1";
  } catch {
    return false;
  }
};

/** Widths of the sidebar and the panel, collapsing, the keyboard shortcuts, and the badges
 *  for access and artifacts. */
export function useLayout(prev: Deps) {
  const { railHidden, setRailHidden, setSurface } = prev;

  /** The search palette, which the topbar's collapsed-sidebar cluster can open (§22). The open
   *  sidebar has its own; this one keeps search from disappearing with it. */
  const [searchOpen, setSearchOpen] = useState(false);
  /** ⌘B collapses the nav out of the grid so the content takes its width; hovering the left
   *  edge brings it back as a floating overlay. Remembered on this device. */
  const [navCollapsed, setNavCollapsed] = useState<boolean>(storedCollapsed);
  const [navPeek, setNavPeek] = useState(false);

  // Both side panels can be dragged, each up to a share of the window so neither can squeeze
  // the transcript out. Each also has to know what the OTHER takes: both could sit inside
  // their own limits and still leave the conversation nothing. A collapsed sidebar or a hidden
  // panel reserves nothing, so closing one gives the other room at once, which is what someone
  // closing a panel wants. NAV_DEFAULT_W matches `--nav-w` in styles.css, but that value only
  // covers the first paint: this hook writes `--nav-w` inline, so this is the default people
  // see, and a width someone dragged (stored under the key) beats both.
  const [navWidth, setNavWidth] = usePanelWidth("assistant:nav-w:v1", {
    fallback: NAV_DEFAULT_W,
    min: NAV_MIN_W,
    maxFraction: 0.34,
    // The panel's minimum rather than its live width, which would make a loop; `fitBeside`
    // below handles the two together properly.
    reserve: CHAT_MIN_W + (railHidden ? 0 : PANEL_MIN_W),
  });
  const [panelWidth, setPanelWidth] = usePanelWidth("assistant:panel-w:v1", {
    fallback: PANEL_DEFAULT_W,
    min: PANEL_MIN_W,
    maxFraction: 0.72,
    reserve: CHAT_MIN_W + (navCollapsed ? 0 : navWidth),
  });
  // The joint limit, applied to the panel since it is the one dragged INTO the conversation;
  // the nav's own ceiling already allows for the panel.
  const winW = useWindowWidth();
  const panelFitted = fitBeside(panelWidth, { winW, other: navCollapsed ? 0 : navWidth, min: PANEL_MIN_W });

  const setNavCollapsedPersist = useCallback((collapsed: boolean) => {
    setNavCollapsed(collapsed);
    try {
      localStorage.setItem(NAV_COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch {
      // Not remembered, but it still applies now.
    }
  }, []);
  const toggleNav = useCallback(() => {
    setNavPeek(false);
    setNavCollapsedPersist(!navCollapsed);
  }, [navCollapsed, setNavCollapsedPersist]);

  // ⌘B toggles the nav; ⌘, opens Settings, the platform's usual shortcut (shown in the account
  // menu, §26); ⌘K opens and closes the search palette, which handles ⌘1–9 and Esc itself but
  // had nothing opening it from the keyboard (UX-REPORT §6). A preview no longer collapses the
  // nav: in the tabbed panel a file is just another tab, so the panel never needs the room.
  useEffect(() => {
    const shortcuts: Record<string, () => void> = {
      b: toggleNav,
      ",": () => setSurface("settings"),
      k: () => setSearchOpen((open) => !open),
    };
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey && !e.ctrlKey) return;
      const run = shortcuts[e.key === "," ? "," : e.key.toLowerCase()];
      if (!run) return;
      e.preventDefault();
      run();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggleNav]);

  /** How many files this conversation has made: the topbar's "Artifacts (N)" when the side panel
   *  is hidden, so made files are never out of sight. */
  const [artifactCount, setArtifactCount] = useState(0);
  /** §32: a link into the panel's Access section (once the session settings drawer). Bumping
   *  this opens the section and scrolls to it; callers also unhide the panel. */
  const [accessKey, setAccessKey] = useState(0);
  const openAccess = () => {
    setRailHidden(false);
    setAccessKey((key) => key + 1);
  };

  // §34 (UX-016): an artifact chip in the transcript has to land somewhere visible. The
  // WorkspacePanel opens a document tab; this makes sure the panel is showing.
  useEffect(() => {
    const show = () => setRailHidden(false);
    window.addEventListener("superuser-open-artifact", show);
    return () => window.removeEventListener("superuser-open-artifact", show);
  }, []);

  return { searchOpen, setSearchOpen, navCollapsed, navPeek, setNavPeek, navWidth, setNavWidth, setPanelWidth, panelFitted, toggleNav, artifactCount, setArtifactCount, accessKey, openAccess };
}
