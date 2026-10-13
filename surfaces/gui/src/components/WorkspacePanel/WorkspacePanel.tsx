import { useEffect, useMemo, useState } from "react";
import type { ArtifactInfo } from "../../api";
import { BoardTab } from "../BoardTab";
import { AccessSection } from "../AccessSection";
import { ActivityTab, filesTouchedBy, type ToolCall } from "../ActivityTab";
import { AssistantTab } from "../AssistantTab";
import { ArtifactViewer, kindFromPath } from "../ArtifactViewer";
import { BrowserTab } from "../BrowserTab";
import { FilesTab } from "../FilesTab";
import { TerminalTab } from "../TerminalTab";
import { panelTabs, type Props, type TabId, type ToolTab } from "./Props";
import { PanelTabStrip } from "./PanelTabStrip";
import { useBrowserSurfacing } from "./useBrowserSurfacing";
import { useArtifactLinks } from "./useArtifactLinks";
import { useOpenDocs } from "./useOpenDocs";
import { useToolPress } from "./useToolPress";

export function WorkspacePanel({
  active,
  sessionId,
  refreshKey,
  todo,
  running,
  personaId,
  workspace,
  branch,
  scratchPrimary,
  openAccessKey,
  openFilesKey,
  openRequest,
  onSend,
  onOpenIntegrations,
  onTabChange,
  onToolsChange,
}: Props) {
  const [tab, setTab] = useState<TabId>("activity");
  // Which tool the topbar has opened, if any. Held here rather than in App because the panel
  // owns its own strip; App only asks.
  const [openTools, setOpenTools] = useState<ToolTab[]>([]);
  // Report the strip's state up, so the control that opened a tool can show it open.
  useEffect(() => {
    onTabChange?.(tab);
  }, [tab, onTabChange]);
  useEffect(() => {
    onToolsChange?.(openTools);
  }, [openTools, onToolsChange]);
  const [artifacts, setArtifacts] = useState<ArtifactInfo[]>([]);
  const [calls, setCalls] = useState<ToolCall[]>([]);
  // A board belongs to a project, so the tab is offered whenever there is one. No
  // poll: this used to ask the server every five seconds whether the board had any
  // items, purely to decide if the tab should exist — a request per session per five
  // seconds to answer a question the presence of a workspace already answers.
  const hasBoard = !!workspace;

  // Files is always present. Every session has a folder on disk
  // (`agent_exec::session_dir_path`) whether or not a project workspace was
  // chosen, and the agent writes its artifacts there. Hiding the tab when
  // `workspace` was empty made those files unreachable — there was no other
  // control that opened one.
  const tabs = panelTabs(openTools, hasBoard);

  // Switching conversations starts the panel over.
  useEffect(() => {
    setCalls([]);
    setTab("activity");
    // The shell and the browser belong to the session that opened them, same as the
    // documents above.
    setOpenTools([]);
  }, [sessionId]);

  useToolPress(openRequest, tab, setTab, openTools, setOpenTools);
  const { docs, contents, openDoc, closeDoc, reloadDoc } = useOpenDocs(sessionId, tab, setTab, refreshKey);

  useArtifactLinks(active, sessionId, artifacts, setArtifacts, openDoc);

  // The §32 deep link — "open Access" from elsewhere in the app selects its tab.
  useEffect(() => {
    if (openAccessKey) setTab("access");
  }, [openAccessKey]);
  useEffect(() => {
    if (openFilesKey) setTab("files");
  }, [openFilesKey]);

  const touched = useMemo(() => filesTouchedBy(calls), [calls]);

  const { wantedUrl, browserUnseen, chosenByHandAt } = useBrowserSurfacing(calls, tab, setTab, active);
  const browserOccluded = !active;

  if (!active) return null;

  return (
    <aside className="right-rail workspace-panel">
      <PanelTabStrip
        tabs={tabs}
        tab={tab}
        onPick={(id) => {
          chosenByHandAt.current = Date.now();
          setTab(id);
        }}
        touchedCount={touched.length}
        running={running}
        browserUnseen={browserUnseen}
        docs={docs}
        onSelectDoc={(path) => setTab(`doc:${path}`)}
        onCloseDoc={closeDoc}
      />

      <div className="wp-body">
        {/* Activity and the documents unmount freely, but Terminal and Browser must
            not: the terminal's scrollback IS the record of what the agent did, and
            remounting the browser would throw away the page. Both stay mounted and
            hide instead. */}
        <div className="wp-pane split" hidden={tab !== "activity"}>
          <ActivityTab
            sessionId={sessionId}
            running={running}
            refreshKey={refreshKey}
            todo={todo}
            onCallsChange={setCalls}
          />
        </div>

        <div className="wp-pane wp-pane-pad" hidden={tab !== "assistant"}>
          <AssistantTab
            sessionId={sessionId}
            personaId={personaId}
            artifacts={artifacts}
            onSend={(t) => onSend?.(t)}
            onViewFiles={() => setTab("files")}
          />
        </div>

        <div className="wp-pane split" hidden={tab !== "files"}>
          <FilesTab
            sessionId={sessionId}
            artifacts={artifacts}
            setArtifacts={setArtifacts}
            touched={touched}
            refreshKey={refreshKey}
            onOpen={openDoc}
          />
        </div>

        <div className="wp-pane wp-pane-fill" hidden={tab !== "terminal"}>
          <TerminalTab cwd={workspace || "."} active={tab === "terminal"} />
        </div>

        <div className="wp-pane" hidden={tab !== "browser"}>
          <BrowserTab
            active={tab === "browser"}
            occluded={browserOccluded}
            workspace={workspace}
            navigateTo={wantedUrl}
          />
        </div>

        <div className="wp-pane" hidden={tab !== "board"}>
          {workspace && <BoardTab workspace={workspace} />}
        </div>

        <div className="wp-pane wp-pane-pad" hidden={tab !== "access"}>
          <AccessSection
            key={sessionId}
            sessionId={sessionId}
            personaId={personaId}
            branch={branch}
            scratchPrimary={scratchPrimary}
            active={tab === "access"}
            openKey={openAccessKey}
            onOpenIntegrations={onOpenIntegrations}
          />
        </div>

        {docs.map((d) => (
          <div className="wp-pane" key={d.path} hidden={tab !== `doc:${d.path}`}>
            <ArtifactViewer
              sessionId={sessionId}
              artifact={d}
              content={contents[d.path] ?? null}
              onReload={() => reloadDoc(d.path)}
              onOpenEntry={(path) =>
                openDoc({
                  path,
                  name: path.split("/").pop() || path,
                  kind: kindFromPath(path),
                  size: 0,
                  modified_at: 0,
                })
              }
            />
          </div>
        ))}
      </div>
    </aside>
  );
}
