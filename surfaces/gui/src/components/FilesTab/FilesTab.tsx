import { useEffect, useMemo, useState } from "react";
import { getArtifacts, readArtifact, revealArtifact, type ArtifactContent, type ArtifactInfo } from "../../api";
import { ArtifactViewer, formatBytes, kindIcon } from "../ArtifactViewer";
import { Icon, ICON } from "../Icon";

export interface TreeNode {
  name: string;
  path: string;
  dir: boolean;
  children: TreeNode[];
  info?: ArtifactInfo;
}

/** Build a directory tree from the flat, workspace-relative paths the server returns. */
export function buildTree(items: ArtifactInfo[]): TreeNode[] {
  const root: TreeNode = { name: "", path: "", dir: true, children: [] };
  for (const a of items) {
    const parts = a.path.split("/").filter(Boolean);
    let node = root;
    parts.forEach((part, i) => {
      const isLeaf = i === parts.length - 1;
      const path = parts.slice(0, i + 1).join("/");
      let next = node.children.find((c) => c.name === part && c.dir === !isLeaf);
      if (!next) {
        next = { name: part, path, dir: !isLeaf, children: [], info: isLeaf ? a : undefined };
        node.children.push(next);
      }
      node = next;
    });
  }
  // Folders first, then files, each alphabetical — the order a file manager uses.
  const sort = (n: TreeNode) => {
    n.children.sort((a, b) => (a.dir === b.dir ? a.name.localeCompare(b.name) : a.dir ? -1 : 1));
    n.children.forEach(sort);
  };
  sort(root);
  return root.children;
}

export function FilesTab({
  sessionId,
  artifacts,
  setArtifacts,
  touched,
  refreshKey,
  onOpen,
}: {
  sessionId: string;
  artifacts: ArtifactInfo[];
  setArtifacts: (a: ArtifactInfo[]) => void;
  /** Workspace-relative paths this run wrote to (from the Activity feed). */
  touched: string[];
  refreshKey: number;
  onOpen: (a: ArtifactInfo) => void;
}) {
  const [selected, setSelected] = useState<ArtifactInfo | null>(null);
  const [content, setContent] = useState<ArtifactContent | null>(null);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const refresh = () =>
    getArtifacts(sessionId)
      .then(setArtifacts)
      .catch(() => setArtifacts([]));

  useEffect(() => {
    refresh();
  }, [sessionId, refreshKey]);

  // The agent just wrote something. Re-read the file being previewed, not only the
  // list — otherwise the panel keeps showing what the file said before the edit, and
  // the user has to reselect it to see the change they just asked for.
  useEffect(() => {
    if (!refreshKey || !selected) return;
    readArtifact(sessionId, selected.path)
      .then((c) => setContent(c))
      .catch(() => {
        /* the file may have been deleted mid-turn; the list refresh will show that */
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  // The preview belongs to the previous session's folder; clear it rather than
  // showing one session's file under another's tree.
  useEffect(() => {
    setSelected(null);
    setContent(null);
  }, [sessionId]);

  const loadPreview = async (a: ArtifactInfo) => {
    setSelected(a);
    setContent(null);
    setContent(await readArtifact(sessionId, a.path).catch(() => null));
  };

  const isTouched = (p: string) => touched.some((t) => t === p || p.endsWith("/" + t) || t.endsWith("/" + p));
  const tree = useMemo(() => buildTree(artifacts), [artifacts]);

  const renderNode = (n: TreeNode, depth: number): React.ReactNode => {
    const pad = { paddingLeft: 6 + depth * 12 };
    if (n.dir) {
      const shut = collapsed[n.path];
      return (
        <div key={n.path}>
          <button
            className="file-row file-row-dir"
            style={pad}
            onClick={() => setCollapsed((c) => ({ ...c, [n.path]: !c[n.path] }))}
          >
            <Icon name={shut ? "chevronRight" : "chevronDown"} size={ICON.sm} />
            <Icon name="folder" size={ICON.md} />
            <span className="file-name">{n.name}</span>
          </button>
          {!shut && n.children.map((c) => renderNode(c, depth + 1))}
        </div>
      );
    }
    const a = n.info!;
    return (
      <button
        key={n.path}
        className={"file-row" + (selected?.path === n.path ? " active" : "")}
        style={pad}
        onClick={() => loadPreview(a)}
        onDoubleClick={() => onOpen(a)}
        title={`${a.path} — double-click to open as a tab`}
      >
        <Icon name={kindIcon(a.kind)} size={ICON.md} />
        <span className="file-name">{n.name}</span>
        {isTouched(a.path) && <span className="file-badge">M</span>}
        <span className="file-meta">{formatBytes(a.size)}</span>
      </button>
    );
  };

  return (
    <div className="files-tab files-split">
      <div className="files-head">
        <span className="rail-muted">
          {artifacts.length} file{artifacts.length === 1 ? "" : "s"}
        </span>
        <div className="rail-actions">
          {artifacts.length > 0 && (
            <button
              className="rail-mini-btn"
              onClick={() => revealArtifact(sessionId, artifacts[0].path, "reveal")}
              title="Show the folder where these files are saved"
            >
              <Icon name="folder" size={ICON.md} />
            </button>
          )}
          <button className="rail-mini-btn" onClick={refresh} title="Refresh">
            <Icon name="refresh" size={ICON.md} />
          </button>
        </div>
      </div>

      {artifacts.length === 0 ? (
        <div className="files-empty">
          <Icon name="folder" size={ICON.lg} />
          <p className="files-empty-line">Nothing here yet</p>
          <p className="files-empty-hint">
            Files the assistant writes — documents, spreadsheets, decks, scripts — appear
            here as it works.
          </p>
        </div>
      ) : (
        <>
          <div className="files-tree">{tree.map((n) => renderNode(n, 0))}</div>
          <div className="files-preview">
            {!selected ? (
              <div className="files-empty">
                <Icon name="file" size={ICON.lg} />
                <p className="files-empty-line">Select a file to preview</p>
                <p className="files-empty-hint">Double-click to open it as its own tab.</p>
              </div>
            ) : (
              <ArtifactViewer
                sessionId={sessionId}
                artifact={selected}
                content={content}
                onReload={async () => {
                  setContent(await readArtifact(sessionId, selected.path).catch(() => null));
                  refresh();
                }}
                onOpenEntry={(path) => {
                  const hit = artifacts.find((a) => a.path === path);
                  if (hit) loadPreview(hit);
                }}
              />
            )}
          </div>
        </>
      )}
    </div>
  );
}
