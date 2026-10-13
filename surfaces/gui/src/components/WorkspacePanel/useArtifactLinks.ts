import { useEffect } from "react";
import { getArtifacts, type ArtifactInfo } from "../../api";
import { kindFromPath } from "../ArtifactViewer";
import { OPEN_ARTIFACT_EVENT } from "../Markdown";

export function useArtifactLinks(
  active: boolean,
  sessionId: string,
  artifacts: ArtifactInfo[],
  setArtifacts: (a: ArtifactInfo[]) => void,
  openDoc: (a: ArtifactInfo) => void,
) {
  // §34 (UX-016): [Title](artifact:path) chips in the transcript open a document tab.
  // Resolve against the loaded list first; on a miss refresh once (the file may be
  // seconds old), then fall back to a minimal record — readArtifact validates the path.
  useEffect(() => {
    if (!active) return;
    const minimal = (path: string): ArtifactInfo => ({
      path,
      name: path.split("/").pop() || path,
      kind: kindFromPath(path),
      size: 0,
      modified_at: 0,
    });
    const match = (list: ArtifactInfo[], path: string) =>
      list.find((a) => a.path === path || a.path.endsWith("/" + path) || a.name === path);
    const onOpen = (e: Event) => {
      const path = String((e as CustomEvent).detail?.path || "");
      if (!path) return;
      const found = match(artifacts, path);
      if (found) return openDoc(found);
      getArtifacts(sessionId)
        .then((list) => {
          setArtifacts(list);
          openDoc(match(list, path) ?? minimal(path));
        })
        .catch(() => openDoc(minimal(path)));
    };
    window.addEventListener(OPEN_ARTIFACT_EVENT, onOpen);
    return () => window.removeEventListener(OPEN_ARTIFACT_EVENT, onOpen);
  }, [active, sessionId, artifacts, openDoc]);
}
