import { useState, useCallback, useEffect } from "react";
import { NAV_COLLAPSED_KEY, NAV_DEFAULT_W, PANEL_MIN_W, PANEL_DEFAULT_W } from "../appHelpers";
import { usePanelWidth, CHAT_MIN_W, useWindowWidth, fitBeside } from "../../components/ResizeHandle";
import type { useAppState } from "./useAppState";
import type { useNavigationState } from "./useNavigationState";

/** Sidebar and panel widths, collapse state, and the access/artifact badges. */
export function useLayout(prev: ReturnType<typeof useAppState> & ReturnType<typeof useNavigationState>) {
  const { railHidden, setSurface, setRailHidden } = prev;
  // The command-palette search, openable from the collapsed-sidebar topbar cluster (§22). The
  // expanded sidebar owns its own instance; this one exists so search never disappears with it.
  const [searchOpen, setSearchOpen] = useState(false);
  // Left-nav collapse (⌘B): when collapsed the sidebar leaves the grid so content reclaims the
  // width; hovering the left edge peeks it back as a floating overlay. Persisted per-device.
  const [navCollapsed, setNavCollapsed] = useState<boolean>(() => {
    try { return localStorage.getItem(NAV_COLLAPSED_KEY) === "1"; } catch { return false; }
  });
  const [navPeek, setNavPeek] = useState(false);
  // Drag-resizable side panels. Ceilings are a fraction of the window so neither can
  // be dragged to a width that leaves no transcript.
  // 240 matches the `--nav-w` default in styles.css. That CSS value is only ever a
  // fallback for the first paint — this hook writes an inline `--nav-w` on the app
  // shell, so THIS number is the default anyone actually sees, and changing the
  // stylesheet alone does nothing. A width someone has dragged is stored under the
  // key and still wins over both.
  // `min` is not a taste call: the top bar has a floor. After the 84px the traffic
  // lights need, its five controls and their gaps come to 129px, plus 4px of right
  // padding — 217px before anything can be clipped. At the old 200 the Workspace
  // icon was simply cut off the edge, which is how it was reported. 224 keeps the
  // 6px of slack `e2e-app/window/` requires.
  // Each panel's ceiling has to know what the OTHER one is taking, or both can be inside
  // their own limits while the conversation between them has nothing left. A collapsed
  // sidebar or a hidden rail reserves nothing, so closing one immediately gives the other
  // room — which is what someone closing a panel is asking for.
  const [navWidth, setNavWidth] = usePanelWidth("assistant:nav-w:v1", {
    fallback: NAV_DEFAULT_W,
    min: 224,
    maxFraction: 0.34,
    // The panel's own minimum, not its current width — the live figure is the other
    // half of a cycle, and `fitBeside` below closes that properly.
    reserve: CHAT_MIN_W + (railHidden ? 0 : PANEL_MIN_W),
  });
  const [panelWidth, setPanelWidth] = usePanelWidth("assistant:panel-w:v1", {
    fallback: PANEL_DEFAULT_W,
    min: PANEL_MIN_W,
    maxFraction: 0.72,
    reserve: CHAT_MIN_W + (navCollapsed ? 0 : navWidth),
  });
  // Both panels are inside their own limits and can still, together, leave nothing. This
  // is the joint constraint, applied to the panel because that is the one you drag INTO
  // the conversation. The nav's own ceiling already accounts for the panel.
  const winW = useWindowWidth();
  const panelFitted = fitBeside(panelWidth, {
    winW,
    other: navCollapsed ? 0 : navWidth,
    min: PANEL_MIN_W,
  });
  const setNavCollapsedPersist = useCallback((v: boolean) => {
    setNavCollapsed(v);
    try { localStorage.setItem(NAV_COLLAPSED_KEY, v ? "1" : "0"); } catch { /* best effort */ }
  }, []);
  const toggleNav = useCallback(() => {
    setNavPeek(false);
    setNavCollapsedPersist(!navCollapsed);
  }, [navCollapsed, setNavCollapsedPersist]);
  // #3 used to collapse the nav whenever a preview opened, because opening a file took
  // the whole rail over and needed the room. In the tabbed panel a file is just another
  // tab, so the panel never grows and the nav can stay where the user put it.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "b") {
        e.preventDefault();
        toggleNav();
      }
      // ⌘, — the platform Settings shortcut (advertised in the account menu, §26).
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        setSurface("settings");
      }
      // ⌘K — the search palette. It supports ⌘1–9 to pick a row and prints its own
      // `Esc` hint, but until now nothing opened it from the keyboard (UX-REPORT §6).
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setSearchOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggleNav]);
  // Count of files this Cowork conversation has produced — surfaces an "Artifacts (N)" button in
  // the topbar when the side panel is hidden, so produced files are never buried.
  const [artifactCount, setArtifactCount] = useState(0);
  // §32 deep link into the rail's Access section (the former Session-settings drawer): bumping
  // the key expands the section and scrolls it into view. Callers also un-hide the rail.
  const [accessKey, setAccessKey] = useState(0);
  const openAccess = () => {
    setRailHidden(false);
    setAccessKey((k) => k + 1);
  };
  // §34 (UX-016): clicking an artifact chip in the transcript must land somewhere visible —
  // WorkspacePanel opens a document tab; this just makes sure the panel isn't hidden.
  useEffect(() => {
    const show = () => setRailHidden(false);
    window.addEventListener("ocw-open-artifact", show);
    return () => window.removeEventListener("ocw-open-artifact", show);
  }, []);

  return { searchOpen, setSearchOpen, navCollapsed, navPeek, setNavPeek, navWidth, setNavWidth, setPanelWidth, panelFitted, toggleNav, artifactCount, setArtifactCount, accessKey, openAccess };
}
