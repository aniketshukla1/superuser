import { Sidebar } from "../components/Sidebar";
import type { AppModel } from "./model/useAppModel";

/** The sidebar, wired to the app's sessions and navigation. */
export function AppSidebar({ m }: { m: AppModel }) {
  const { agent, workspace, surfaces, sessions, projects, sessionId, switchAgent, startNewSession,
      selectSession, newProject, newSessionInProject, renameConversation, deleteConversation,
      toggleArchived, togglePinned, openSettings, openPersona, setSurface, setScheduledOpenId,
      setScheduledNew, surface, navCollapsed, toggleNav, setNavPeek } = m;
  return (
    <Sidebar
        agent={agent}
        workspace={workspace || ""}
        surfaces={surfaces}
        sessions={sessions}
        projects={projects}
        activeSession={sessionId}
        onSwitchAgent={switchAgent}
        onNewSession={startNewSession}
        onSelectSession={selectSession}
        onNewProject={newProject}
        onNewInProject={newSessionInProject}
        onRenameSession={renameConversation}
        onDeleteSession={deleteConversation}
        onArchiveSession={toggleArchived}
        onTogglePin={togglePinned}
        onManage={() => openSettings("appearance")}
        onOpenPersona={(id) => {
          openPersona(id, "session");
        }}
        onManagePersonas={() => openSettings("personas")}
        onOpenScheduled={() => setSurface("scheduled")}
        onOpenAutomation={(id) => {
          setScheduledOpenId(id);
          setSurface("scheduled");
        }}
        onNewAutomation={() => {
          setScheduledOpenId(null);
          setScheduledNew((n) => n + 1);
          setSurface("scheduled");
        }}
        onOpenIntegrations={() => setSurface("integrations")}
        onOpenAudit={() => setSurface("audit")}
        onOpenInbox={() => setSurface("inbox")}
        onOpenBrain={() => setSurface("brain")}
        scheduledActive={surface === "scheduled"}
        integrationsActive={surface === "integrations"}
        auditActive={surface === "audit"}
        inboxActive={surface === "inbox"}
        brainActive={surface === "brain"}
        collapsed={navCollapsed}
        onCollapse={toggleNav}
        onPeekLeave={() => setNavPeek(false)}
      />
  );
}
