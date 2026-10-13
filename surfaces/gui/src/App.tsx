import { Notices } from "./components/Notices";
import { UpdateBanner } from "./components/UpdateBanner";
import { SecretsLockedNotice } from "./components/SecretsLockedNotice";
import { Icon, ICON } from "./components/Icon";
import { ResizeHandle } from "./components/ResizeHandle";
import { NAV_DEFAULT_W, PANEL_DEFAULT_W } from "./App/appHelpers";
import { SurfaceBoundary } from "./components/SurfaceBoundary";
import { AutomationsSurface } from "./components/WatchesView";
import { IntegrationsView } from "./components/IntegrationsView";
import { BrainView } from "./components/BrainView";
import { AuditView } from "./components/AuditView";
import { InboxView } from "./components/InboxView";
import { PersonaView } from "./components/PersonaView";
import { LiveView } from "./components/LiveView";
import { WorkspaceTrustPrompt } from "./components/WorkspaceTrustPrompt";
import { useAppModel } from "./App/model/useAppModel";
import { RunToast } from "./App/RunToast";
import { AppOnboarding } from "./App/AppOnboarding";
import { AppSidebar } from "./App/AppSidebar";
import { SettingsSurface } from "./App/SettingsSurface";
import { Topbar } from "./App/Topbar";
import { MainChat } from "./App/MainChat";
import { AppWorkspacePanel } from "./App/AppWorkspacePanel";
import { AppSearch } from "./App/AppSearch";
import { AppFolderGate } from "./App/AppFolderGate";
import { renderSplash } from "./App/render/splash";

export function App() {
  const m = useAppModel();
  const { appReady, overlay, navCollapsed, navPeek, navWidth, panelFitted, simOverlay, secrets,
    setSecrets, setNavPeek, surface, toggleNav, setNavWidth, route, openRunSession, runTaskNow,
    scheduledOpenId, scheduledNew, navigate, openSessionFromInbox, personaViewId, agent,
    personaViewReturn, openSettings, setSurface, railHidden, setPanelWidth, liveView, nameOf,
    setLiveView, workspaceTrustRequest, setWorkspaceTrustRequest } = m;
  return !appReady ? <>{null}{renderSplash(m)}</> : (
    <>
    <div
      className={
        "app" +
        (overlay ? " tauri-overlay" : "") +
        (navCollapsed ? " nav-collapsed" : "") +
        (navCollapsed && navPeek ? " nav-peek" : "")
      }
      // Both side panels are drag-resizable; the widths drive the grid column and the
      // panel's own width, so the layout follows from one place.
      style={{ ["--nav-w" as string]: `${navWidth}px`, ["--wp-w" as string]: `${panelFitted}px` }}
    >
      {/* Dev-only fake traffic lights so ?overlay=1 previews the real desktop top-left. */}
      {simOverlay && (
        <div className="sim-traffic-lights" aria-hidden="true">
          <span /><span /><span />
        </div>
      )}
      {/* Failed writes, wherever they happen (UX-REPORT §5). */}
      <Notices />
      {/* Desktop-only auto-update prompt (15s after boot, then every 30 min; inert in browser). */}
      <UpdateBanner />
      {/* Saved keys that won't open: said, with the way out, instead of every key reading as missing. */}
      <SecretsLockedNotice status={secrets} onResolved={() => setSecrets("empty")} />
      {/* UX-026: automation-start toast — quiet panel, neutral dot/drain, accent only
          on the action (rev 2); auto-dismisses with the 5s drain bar. */}
      <RunToast m={m} />
      {/* When collapsed, a thin left-edge zone peeks the nav back as a floating overlay. */}
      {navCollapsed && (
        <div
          className="nav-hover-zone"
          onMouseEnter={() => setNavPeek(true)}
          aria-hidden="true"
        />
      )}
      {/* Explicit reveal affordance while collapsed (alongside hover-peek + ⌘B) — on every
          surface EXCEPT the session view, whose topbar carries the [sidebar][+][search] cluster
          instead (§22; no duplicate reveal buttons). */}
      {navCollapsed && !navPeek && surface !== "session" && (
        <button
          className="nav-reveal-btn"
          onClick={toggleNav}
          onMouseEnter={() => setNavPeek(true)}
          title="Show sidebar (⌘B)"
          aria-label="Show sidebar"
        >
          <Icon name="sidebar" size={ICON.md} />
        </button>
      )}
      <AppOnboarding m={m} />
      <AppSidebar m={m} />
      {/* Nothing to grab while the nav is collapsed — it's off-screen, and the peek
          overlay is transient. */}
      {!navCollapsed && (
        <ResizeHandle side="left" width={navWidth} onResize={setNavWidth} label="Resize the sidebar" defaultWidth={NAV_DEFAULT_W} />
      )}
      {/* One surface crashing must not take the shell with it (UX-REPORT §1): the
          boundary is inside the sidebar's sibling slot, and clears when the user
          navigates somewhere else. */}
      <SurfaceBoundary resetKey={`${surface}:${route.tab ?? ""}`}>
      {surface === "scheduled" ? (
        <AutomationsSurface
          onOpenRun={openRunSession}
          onRunNow={runTaskNow}
          initialOpenId={scheduledOpenId}
          newRequest={scheduledNew}
          tab={route.tab}
          // `replace`: switching tabs corrects where you are, it isn't a place you
          // navigated to. Pushing would make Back undo a tab click one at a time.
          onTabChange={(t) => navigate({ surface: "scheduled", tab: t }, true)}
        />
      ) : surface === "integrations" ? (
        <IntegrationsView />
      ) : surface === "settings" ? (
        <SettingsSurface m={m} />
      ) : surface === "brain" ? (
        <BrainView
          section={route.tab}
          onSectionChange={(s) => navigate({ surface: "brain", tab: s }, true)}
        />
      ) : surface === "audit" ? (
        <AuditView />
      ) : surface === "inbox" ? (
        <InboxView onOpenSession={openSessionFromInbox} />
      ) : surface === "persona" ? (
        <PersonaView
          personaId={personaViewId || agent}
          onBack={() =>
            personaViewReturn === "settings" ? openSettings("personas") : setSurface("session")
          }
          onOpenIntegrations={() => setSurface("integrations")}
        />
      ) : (
      // The panel now renders for every persona (`chat` included, where the old rail was
      // hidden outright), so the inset must follow the same condition — otherwise the
      // transcript runs underneath it.
      <div className={"main" + (surface === "session" && !railHidden ? " rail-open" : "")}>
        <Topbar m={m} />
        <div className={"main-workspace" + (railHidden ? " rail-hidden" : "")}>
          {/* The session surface's reading column is the page's main landmark; it was the
              one surface without one (UX-REPORT §8). */}
          <MainChat m={m} />
          {/* The panel now carries a terminal and a browser, which are useful whoever
              you're talking to — so it renders for `chat` too, where the old rail was
              hidden outright. Files still needs somewhere to look. */}
          <AppWorkspacePanel m={m} />
          {surface === "session" && !railHidden && (
            <ResizeHandle
              side="right"
              width={panelFitted}
              onResize={setPanelWidth}
              label="Resize the workspace panel"
              defaultWidth={PANEL_DEFAULT_W}
            />
          )}
        </div>
      </div>
      )}
      </SurfaceBoundary>

      {/* Search from the collapsed-sidebar topbar cluster (the sidebar's own instance is
          unreachable while it's collapsed). */}
      <AppSearch m={m} />

      {liveView && <LiveView persona={liveView} name={nameOf(liveView)} onClose={() => setLiveView(null)} />}

      <AppFolderGate m={m} />
      {workspaceTrustRequest && (
        <WorkspaceTrustPrompt
          request={workspaceTrustRequest}
          onClose={() => setWorkspaceTrustRequest(null)}
        />
      )}
    </div>
    {renderSplash(m)}
    </>
  );
}
