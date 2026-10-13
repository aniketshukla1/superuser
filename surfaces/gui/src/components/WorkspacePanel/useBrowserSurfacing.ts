import { useEffect, useMemo, useRef, useState } from "react";
import { browserVisible, isTauri } from "../../tauri";
import { browserUrlWantedBy, shouldSurfaceBrowser, type ToolCall } from "../ActivityTab";
import type { TabId } from "./Props";

/** The Browser tab opens itself.
 *
 * When the agent shows a page, the point is that the user watches it happen. But an
 * auto-opening tab that interrupts is worse than one that never opens, so surfacing it is
 * subject to two rules, and the *page* loads either way — only the tab switch is withheld.
 * Nothing is lost by being polite: the page is already there when the user does look, and
 * the tab carries a dot until then. */
export function useBrowserSurfacing(calls: ToolCall[], tab: TabId, setTab: (t: TabId) => void, active: boolean) {
  const wantedUrl = useMemo(() => browserUrlWantedBy(calls), [calls]);
  /** When the user last chose a tab by hand. We don't fight a deliberate choice. */
  const chosenByHandAt = useRef(0);
  const lastSurfaced = useRef("");
  const [browserUnseen, setBrowserUnseen] = useState(false);

  useEffect(() => {
    if (!wantedUrl || wantedUrl === lastSurfaced.current) return;
    lastSurfaced.current = wantedUrl;
    const el = document.activeElement as HTMLElement | null;
    const surface = shouldSurfaceBrowser({
      alreadyWatching: tab === "browser",
      typing: !!el && (el.tagName === "TEXTAREA" || el.tagName === "INPUT" || el.isContentEditable),
      msSinceManualChoice: Date.now() - chosenByHandAt.current,
    });
    if (surface) setTab("browser");
    else if (tab !== "browser") setBrowserUnseen(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wantedUrl]);

  // Looking at it clears the "there's something here" mark.
  useEffect(() => {
    if (tab === "browser") setBrowserUnseen(false);
  }, [tab]);

  // The browser is a native view above the HTML: it can't be covered by another tab
  // or by anything the app draws. It has to be taken off screen.
  useEffect(() => {
    if (isTauri()) browserVisible(active && tab === "browser");
  }, [active, tab]);

  return { wantedUrl, browserUnseen, chosenByHandAt };
}
