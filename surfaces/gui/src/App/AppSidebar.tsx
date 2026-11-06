import { Sidebar } from "../components/Sidebar";
import type { RouteSurface } from "../route";
import type { AppModel } from "./model/useAppModel";

/** The sidebar, connected to the app's sessions and navigation. */
export function AppSidebar({ m }: { m: AppModel }) {
  const { setScheduledOpenId, setSurface, surface } = m;
  const go = (to: RouteSurface) => () => setSurface(to);
  const openAutomation = (id: string) => {
    setScheduledOpenId(id);
    setSurface("scheduled");
  };
  const newAutomation = () => {
    setScheduledOpenId(null);
    m.setScheduledNew((count) => count + 1);
    setSurface("scheduled");
  };
  return (
    <Sidebar
      agent={m.agent}
      workspace={m.workspace || ""}
      surfaces={m.surfaces}
      sessions={m.sessions}
      projects={m.projects}
      activeSession={m.sessionId}
      collapsed={m.navCollapsed}
      onCollapse={m.toggleNav}
      onPeekLeave={() => m.setNavPeek(false)}
      onSwitchAgent={m.switchAgent}
      onSelectSession={m.selectSession}
      onNewSession={m.startNewSession}
      onNewProject={m.newProject}
      onNewInProject={m.newSessionInProject}
      onRenameSession={m.renameConversation}
      onDeleteSession={m.deleteConversation}
      onArchiveSession={m.toggleArchived}
      onTogglePin={m.togglePinned}
      onManage={() => m.openSettings("appearance")}
      onManagePersonas={() => m.openSettings("personas")}
      onOpenPersona={(id) => m.openPersona(id, "session")}
      onOpenAutomation={openAutomation}
      onNewAutomation={newAutomation}
      onOpenScheduled={go("scheduled")}
      onOpenIntegrations={go("integrations")}
      onOpenAudit={go("audit")}
      onOpenInbox={go("inbox")}
      onOpenBrain={go("brain")}
      scheduledActive={surface === "scheduled"}
      integrationsActive={surface === "integrations"}
      auditActive={surface === "audit"}
      inboxActive={surface === "inbox"}
      brainActive={surface === "brain"}
    />
  );
}
