import { WorkspacePanel } from "../components/WorkspacePanel";
import type { AppModel } from "./model/useAppModel";

/** The workspace panel beside the chat. */
export function AppWorkspacePanel({ m }: { m: AppModel }) {
  const { surface, railHidden, sessionId, browserRefreshKey, panelRequest, setPanelTab, todo,
      running, agent, send, workspace, branch, gatesWorkspace, accessKey, filesRequest,
      setLiveTools, setSurface } = m;
  return (
    <WorkspacePanel
            active={surface === "session" && !railHidden}
            sessionId={sessionId}
            refreshKey={browserRefreshKey}
            openRequest={panelRequest}
            onTabChange={setPanelTab}
            todo={todo}
            running={running}
            personaId={agent}
            onSend={(t) => send(t)}
            workspace={workspace || undefined}
            branch={branch}
            scratchPrimary={!gatesWorkspace(agent)}
            openAccessKey={accessKey}
            openFilesKey={filesRequest}
            onToolsChange={setLiveTools}
            onOpenIntegrations={() => setSurface("integrations")}
          />
  );
}
