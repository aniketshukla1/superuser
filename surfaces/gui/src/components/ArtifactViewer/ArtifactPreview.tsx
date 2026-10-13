import type { ArtifactContent, ArtifactInfo } from "../../api";
import { revealArtifact } from "../../api";
import { DocEditor } from "../DocEditor";
import { SlideViewer } from "../SlideViewer";
import { UniverSheet } from "../UniverSheet";
import { Icon, ICON } from "../Icon";
import { Markdown } from "../Markdown";
import { Chart, type ChartSpec } from "../Chart";
import { formatBytes } from "./kindLabel";
import { PdfViewer } from "./PdfViewer";

/** The file, rendered for its kind — the viewer's body when it is not being edited. */
export function ArtifactPreview({
  sessionId,
  artifact,
  content,
  chartSpec,
  reloadKey,
  onOpenEntry,
}: {
  sessionId: string;
  artifact: ArtifactInfo;
  content: ArtifactContent | null;
  chartSpec: ChartSpec | null;
  reloadKey: number;
  onOpenEntry?: (path: string) => void;
}) {
  return (
    !content ? (
      <div className="rail-muted">Loading...</div>
    ) : content.error ? (
      <div className="rail-error">{content.error}</div>
    ) : content.kind === "html" ? (
      <iframe
        key={`${artifact.path}-${reloadKey}`}
        sandbox="allow-scripts allow-same-origin"
        className="artifact-frame"
        srcDoc={content.content || ""}
      />
    ) : chartSpec ? (
      <div className="artifact-md">
        <Chart spec={chartSpec} />
      </div>
    ) : content.kind === "markdown" ? (
      <div className="artifact-md">
        <Markdown text={content.content || ""} />
      </div>
    ) : content.kind === "slides" ? (
      <SlideViewer dataUrl={content.data_url || ""} name={artifact.name} />
    ) : content.kind === "doc" ? (
      <DocEditor
        paragraphs={content.paragraphs || []}
        dataUrl={content.data_url}
        name={artifact.name}
      />
    ) : content.kind === "image" ? (
      <img className="artifact-image" src={content.data_url} />
    ) : content.kind === "pdf" ? (
      <PdfViewer dataUrl={content.data_url || ""} />
    ) : content.kind === "csv" ? (
      // A CSV is a grid. It used to render as a static table and edit as raw
      // text — a worse tool than the one already sitting beside it.
      <UniverSheet
        dataUrl={content.data_url || `data:text/csv;base64,${btoa(unescape(encodeURIComponent(content.content || "")))}`}
        sessionId={sessionId}
        path={artifact.path}
        format="csv"
      />
    ) : content.kind === "sheet" ? (
      <UniverSheet dataUrl={content.data_url || ""} sessionId={sessionId} path={artifact.path} />
    ) : content.kind === "folder" ? (
      // A linked directory (e.g. a skill package): render the listing, click through.
      <div className="artifact-folderlist" data-testid="artifact-folder">
        {(content.entries || []).map((e) => (
          <button
            key={e.name}
            className="artifact-folder-row"
            onClick={() => onOpenEntry?.(`${artifact.path.replace(/\/+$/, "")}/${e.name}`)}
          >
            <Icon name={e.dir ? "folder" : "file"} size={ICON.md} />
            <span className="artifact-folder-name">{e.name}</span>
            {!e.dir && <span className="artifact-folder-size">{formatBytes(e.size)}</span>}
          </button>
        ))}
        {!content.entries?.length && <div className="rail-muted">This folder is empty.</div>}
      </div>
    ) : content.kind === "office" ? (
      <div className="artifact-open-prompt">
        <Icon name="panelOpen" size={ICON.xl} />
        <p>This {/\.pptx?$/i.test(artifact.name) ? "PowerPoint" : "Word"} file can’t be previewed here.</p>
        <button className="btn sm" onClick={() => revealArtifact(sessionId, artifact.path, "open")}>
          Open in default app
        </button>
      </div>
    ) : (
      <pre className="artifact-code">{content.content}</pre>
    )
  );
}
