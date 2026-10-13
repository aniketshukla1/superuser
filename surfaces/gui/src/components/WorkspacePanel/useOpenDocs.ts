import { useCallback, useEffect, useState } from "react";
import { readArtifact, type ArtifactContent, type ArtifactInfo } from "../../api";
import { setOpenDocuments } from "../../tauri";
import type { TabId } from "./Props";

/** The documents open as tabs in the panel, and their contents. */
export function useOpenDocs(
  sessionId: string,
  tab: TabId,
  setTab: (t: TabId | ((cur: TabId) => TabId)) => void,
  refreshKey?: number,
) {
  const [docs, setDocs] = useState<ArtifactInfo[]>([]);
  const [contents, setContents] = useState<Record<string, ArtifactContent | null>>({});

  // Switching conversations closes every document — they belong to the previous
  // session's workspace, which the new one can't (and shouldn't) read.
  useEffect(() => {
    setDocs([]);
    setContents({});
  }, [sessionId]);

  const openDoc = useCallback((a: ArtifactInfo) => {
    setDocs((prev) => (prev.some((d) => d.path === a.path) ? prev : [...prev, a]));
    setTab(`doc:${a.path}`);
  }, [setTab]);

  const closeDoc = (path: string) => {
    setDocs((prev) => prev.filter((d) => d.path !== path));
    setContents((prev) => {
      const next = { ...prev };
      delete next[path];
      return next;
    });
    setTab((cur) => (cur === `doc:${path}` ? "activity" : cur));
  };

  // Load a document the first time its tab is shown.
  const openPath = tab.startsWith("doc:") ? tab.slice(4) : "";
  useEffect(() => {
    if (!openPath || openPath in contents) return;
    setContents((prev) => ({ ...prev, [openPath]: null }));
    readArtifact(sessionId, openPath)
      .then((c) => setContents((prev) => ({ ...prev, [openPath]: c })))
      .catch(() => setContents((prev) => ({ ...prev, [openPath]: null })));
  }, [openPath, sessionId]);

  // The panel is the only thing that knows what the user is looking at, so it is
  // the only thing that can tell the session. Reported on every change rather than
  // read at turn start: a turn begins in the backend, which cannot ask the window.
  useEffect(() => {
    const activePath = tab.startsWith("doc:") ? tab.slice(4) : null;
    void setOpenDocuments(sessionId, docs.map((d) => d.path), activePath).catch(() => {});
  }, [sessionId, docs, tab]);

  // A document the agent edited is stale on screen until it is re-read. Refreshing
  // on turn completion is what makes an edit visible where the user is already
  // looking, instead of behind a reload they have to think to do.
  useEffect(() => {
    if (!refreshKey) return;
    for (const d of docs) void reloadDoc(d.path);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  const reloadDoc = async (path: string) => {
    setContents((prev) => ({ ...prev, [path]: null }));
    const c = await readArtifact(sessionId, path).catch(() => null);
    setContents((prev) => ({ ...prev, [path]: c }));
  };

  return { docs, contents, openDoc, closeDoc, reloadDoc };
}
