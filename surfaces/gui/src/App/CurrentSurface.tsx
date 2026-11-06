import { AuditView } from "../components/AuditView";
import { BrainView } from "../components/BrainView";
import { InboxView } from "../components/InboxView";
import { IntegrationsView } from "../components/IntegrationsView";
import { PersonaView } from "../components/PersonaView";
import { ResizeHandle } from "../components/ResizeHandle";
import { AutomationsSurface } from "../components/WatchesView";
import { AppWorkspacePanel } from "./AppWorkspacePanel";
import { PANEL_DEFAULT_W } from "./appHelpers";
import { MainChat } from "./MainChat";
import { SettingsSurface } from "./SettingsSurface";
import { Topbar } from "./Topbar";
import type { AppModel } from "./model/useAppModel";

/** Whatever the route points at. Anything that isn't one of the named surfaces is the session. */
export function CurrentSurface({ m }: { m: AppModel }) {
  const { surface, route, navigate } = m;
  switch (surface) {
    case "scheduled":
      return (
        <AutomationsSurface
          onOpenRun={m.openRunSession}
          onRunNow={m.runTaskNow}
          initialOpenId={m.scheduledOpenId}
          newRequest={m.scheduledNew}
          tab={route.tab}
          // Replace, don't push: a tab change adjusts where you are rather than going
          // somewhere, and pushing would make Back step through tab clicks one at a time.
          onTabChange={(tab) => navigate({ surface: "scheduled", tab }, true)}
        />
      );
    case "integrations":
      return <IntegrationsView />;
    case "settings":
      return <SettingsSurface m={m} />;
    case "brain":
      return <BrainView section={route.tab} onSectionChange={(tab) => navigate({ surface: "brain", tab }, true)} />;
    case "audit":
      return <AuditView />;
    case "inbox":
      return <InboxView onOpenSession={m.openSessionFromInbox} />;
    case "persona":
      return (
        <PersonaView
          personaId={m.personaViewId || m.agent}
          onBack={() => (m.personaViewReturn === "settings" ? m.openSettings("personas") : m.setSurface("session"))}
          onOpenIntegrations={() => m.setSurface("integrations")}
        />
      );
    default:
      return <SessionSurface m={m} />;
  }
}

/** The conversation, its topbar and the workspace panel. The panel shows for every assistant,
 *  `chat` included, because its terminal and browser help whoever you are talking to; the
 *  main column makes room for it under the same condition, or the transcript would run
 *  underneath it. */
function SessionSurface({ m }: { m: AppModel }) {
  const railShown = !m.railHidden;
  const onSession = m.surface === "session";
  return (
    <div className={onSession && railShown ? "main rail-open" : "main"}>
      <Topbar m={m} />
      <div className={railShown ? "main-workspace" : "main-workspace rail-hidden"}>
        {/* The reading column is the page's main landmark (UX-REPORT §8). */}
        <MainChat m={m} />
        <AppWorkspacePanel m={m} />
        {onSession && railShown && (
          <ResizeHandle
            side="right"
            width={m.panelFitted}
            onResize={m.setPanelWidth}
            label="Resize the workspace panel"
            defaultWidth={PANEL_DEFAULT_W}
          />
        )}
      </div>
    </div>
  );
}
