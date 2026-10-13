import { useCallback, useEffect, useRef, useState } from "react";
import { browserBounds, browserHistory, browserNavigate, browserOpen, browserVisible, isTauri, onBrowserNav, openExternal } from "../../tauri";
import { Icon, ICON } from "../Icon";
import { PANEL_DRAG_EVENT } from "../ResizeHandle";
import { BrowserStartPage } from "./BrowserStartPage";
import { useDevServers } from "./useDevServers";

export function BrowserTab({
  active,
  occluded,
  workspace,
  navigateTo,
}: {
  active: boolean;
  occluded: boolean;
  workspace?: string;
  /** A URL the agent asked to show. Loads whether or not this tab is on screen,
   *  so opening the tab later finds the page already there. */
  navigateTo?: string;
}) {
  // Where the webview should sit: this element is an empty spacer whose only job is
  // to be measured.
  const slot = useRef<HTMLDivElement | null>(null);
  const [address, setAddress] = useState("");
  const [current, setCurrent] = useState("");
  const [error, setError] = useState("");
  const [dragging, setDragging] = useState(false);
  const opened = useRef(false);

  // Nothing is loaded yet → the start page shows and the webview must stay away.
  const showingStart = !current;
  const shouldShow = active && !occluded && !dragging && !showingStart;

  const measure = useCallback(() => {
    const el = slot.current;
    if (!el) return null;
    const r = el.getBoundingClientRect();
    // Logical (CSS) pixels — Tauri converts to physical using the monitor's scale.
    return { x: Math.round(r.left), y: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) };
  }, []);

  // A panel drag would die the moment the cursor crossed the native webview, so the
  // webview steps aside for the duration.
  useEffect(() => {
    const onDrag = (e: Event) => setDragging(!!(e as CustomEvent).detail?.active);
    window.addEventListener(PANEL_DRAG_EVENT, onDrag);
    return () => window.removeEventListener(PANEL_DRAG_EVENT, onDrag);
  }, []);

  // Create on first navigation, then keep it parked at the slot's rect.
  useEffect(() => {
    if (!isTauri() || !shouldShow) return;
    const rect = measure();
    if (!rect || rect.w < 2 || rect.h < 2) return;
    if (opened.current) {
      browserBounds(rect);
      browserVisible(true);
      return;
    }
    opened.current = true;
    browserOpen(current, rect).catch((e) => setError(String((e as Error)?.message || e)));
  }, [shouldShow, current, measure]);

  // Hide rather than destroy whenever this isn't the visible tab, something is
  // layered over the panel, or the start page is showing. The page stays alive.
  useEffect(() => {
    if (!isTauri() || !opened.current) return;
    browserVisible(shouldShow);
  }, [shouldShow]);

  // Follow the slot through window resizes, sidebar collapses, and panel drags.
  useEffect(() => {
    if (!isTauri() || !slot.current) return;
    const sync = () => {
      if (!shouldShow) return;
      const rect = measure();
      if (rect && rect.w > 1 && rect.h > 1) browserBounds(rect);
    };
    const ro = new ResizeObserver(sync);
    ro.observe(slot.current);
    window.addEventListener("resize", sync);
    // Scrolling an ancestor moves the slot without resizing it.
    window.addEventListener("scroll", sync, true);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", sync);
      window.removeEventListener("scroll", sync, true);
    };
  }, [shouldShow, measure]);

  // The page navigates itself constantly (redirects, links, form posts). The address
  // bar should follow the page, not the last thing that was typed.
  useEffect(() => {
    // Same async-unsubscribe race as the devserver listeners above.
    let cancelled = false;
    let un: (() => void) | undefined;
    onBrowserNav((url) => {
      setCurrent(url);
      setAddress(url);
    })
      .then((u) => (cancelled ? u() : (un = u)))
      .catch(() => {});
    return () => {
      cancelled = true;
      un?.();
    };
  }, []);

  const go = (raw: string) => {
    if (!raw.trim()) return;
    setError("");
    const nav = opened.current ? browserNavigate(raw) : Promise.resolve(raw);
    nav
      .then((url) => {
        setCurrent(url || raw);
        setAddress(url || raw);
      })
      .catch((e) => setError(String((e as Error)?.message || e)));
  };

  const { project, pickProject, servers, starting, running, logs, openLog, setOpenLog, runServer, stopServer } =
    useDevServers(workspace, go, setError);

  // The agent asked for a page. Load it even while this tab is hidden — when the
  // webview isn't open yet `go` just records it, and the open effect picks it up on
  // first show. Whether the panel *switches* to this tab is the panel's decision,
  // not ours: see the focus guards in WorkspacePanel.
  const lastAgentNav = useRef("");
  useEffect(() => {
    if (!navigateTo || navigateTo === lastAgentNav.current) return;
    lastAgentNav.current = navigateTo;
    go(navigateTo);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigateTo]);


  return (
    <div className="browser-tab">
      <div className="browser-bar">
        <button className="artifact-icon-btn" onClick={() => browserHistory("back")} title="Back" aria-label="Back">
          <Icon name="arrowLeft" size={ICON.md} />
        </button>
        <button
          className="artifact-icon-btn"
          onClick={() => browserHistory("forward")}
          title="Forward"
          aria-label="Forward"
        >
          <Icon name="arrowLeft" size={ICON.md} className="flip-x" />
        </button>
        <button
          className="artifact-icon-btn"
          onClick={() => (showingStart ? undefined : browserHistory("reload"))}
          title="Reload"
          aria-label="Reload"
        >
          <Icon name="refresh" size={ICON.md} />
        </button>
        <form
          className="browser-address"
          onSubmit={(e) => {
            e.preventDefault();
            go(address);
          }}
        >
          <input
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            onFocus={(e) => e.currentTarget.select()}
            placeholder="Type a URL"
            spellCheck={false}
            aria-label="Address"
          />
        </form>
        <button
          className="artifact-icon-btn"
          onClick={() => current && openExternal(current)}
          title="Open in your default browser"
          aria-label="Open externally"
        >
          <Icon name="panelOpen" size={ICON.md} />
        </button>
      </div>
      {error && <div className="rail-error browser-error">{error}</div>}

      {showingStart ? (
        <BrowserStartPage
          servers={servers}
          running={running}
          starting={starting}
          logs={logs}
          openLog={openLog}
          setOpenLog={setOpenLog}
          project={project}
          pickProject={pickProject}
          runServer={runServer}
          stopServer={stopServer}
        />
      ) : (
        // The webview covers this. It stays empty on purpose — anything rendered here
        // would be hidden behind a native view.
        <div className="browser-slot" ref={slot} data-testid="browser-slot" />
      )}
    </div>
  );
}
