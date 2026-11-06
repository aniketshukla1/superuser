import { LiveView } from "./components/LiveView";
import { Notices } from "./components/Notices";
import { ResizeHandle } from "./components/ResizeHandle";
import { SecretsLockedNotice } from "./components/SecretsLockedNotice";
import { SurfaceBoundary } from "./components/SurfaceBoundary";
import { UpdateBanner } from "./components/UpdateBanner";
import { WorkspaceTrustPrompt } from "./components/WorkspaceTrustPrompt";
import { AppFolderGate } from "./App/AppFolderGate";
import { AppOnboarding } from "./App/AppOnboarding";
import { AppSearch } from "./App/AppSearch";
import { AppSidebar } from "./App/AppSidebar";
import { NAV_DEFAULT_W } from "./App/appHelpers";
import { CollapsedNav } from "./App/CollapsedNav";
import { CurrentSurface } from "./App/CurrentSurface";
import { RunToast } from "./App/RunToast";
import { useAppModel } from "./App/model/useAppModel";
import { renderSplash } from "./App/render/splash";

/** The shell's classes: the desktop title-bar overlay, and the nav collapsed or peeking. */
function shellClass(overlay: boolean, collapsed: boolean, peeking: boolean): string {
  const classes = ["app"];
  if (overlay) classes.push("tauri-overlay");
  if (collapsed) classes.push("nav-collapsed");
  if (collapsed && peeking) classes.push("nav-peek");
  return classes.join(" ");
}

export function App() {
  const m = useAppModel();
  if (!m.appReady) return <>{null}{renderSplash(m)}</>;
  // Both side panels can be dragged to any width. The two widths feed the grid column and the
  // panel's own width, so the whole layout follows from them.
  const widths = { ["--nav-w" as string]: `${m.navWidth}px`, ["--wp-w" as string]: `${m.panelFitted}px` };
  return (
    <>
      <div className={shellClass(m.overlay, m.navCollapsed, m.navPeek)} style={widths}>
        {/* In development, ?overlay=1 draws stand-in window buttons to preview the desktop's corner. */}
        {m.simOverlay && (
          <div className="sim-traffic-lights" aria-hidden="true">
            <span />
            <span />
            <span />
          </div>
        )}
        {/* Writes that failed, from anywhere in the app (UX-REPORT §5). */}
        <Notices />
        {/* Desktop only: offers an update 15 s after launch, then every 30 minutes. */}
        <UpdateBanner />
        {/* Keys that are saved but won't open say so, with the way out, instead of reading as missing. */}
        <SecretsLockedNotice status={m.secrets} onResolved={() => m.setSecrets("empty")} />
        {/* UX-026: the toast when an automation starts, gone with its 5 s drain bar (rev 2). */}
        <RunToast m={m} />
        <CollapsedNav m={m} />
        <AppOnboarding m={m} />
        <AppSidebar m={m} />
        {/* A collapsed nav is off-screen and its peek is fleeting: no edge to drag then. */}
        {!m.navCollapsed && (
          <ResizeHandle
            side="left"
            width={m.navWidth}
            onResize={m.setNavWidth}
            label="Resize the sidebar"
            defaultWidth={NAV_DEFAULT_W}
          />
        )}
        {/* A surface that crashes must not take the shell with it (UX-REPORT §1). The boundary
            resets when the user goes somewhere else. */}
        <SurfaceBoundary resetKey={`${m.surface}:${m.route.tab ?? ""}`}>
          <CurrentSurface m={m} />
        </SurfaceBoundary>
        {/* Search for the topbar's collapsed-sidebar cluster; the sidebar's own is out of reach then. */}
        <AppSearch m={m} />
        {m.liveView && <LiveView persona={m.liveView} name={m.nameOf(m.liveView)} onClose={() => m.setLiveView(null)} />}
        <AppFolderGate m={m} />
        {m.workspaceTrustRequest && (
          <WorkspaceTrustPrompt request={m.workspaceTrustRequest} onClose={() => m.setWorkspaceTrustRequest(null)} />
        )}
      </div>
      {renderSplash(m)}
    </>
  );
}
