import { useEffect, useState, useMemo } from "react";
import { revealArtifact, writeArtifact, type ArtifactContent, type ArtifactInfo } from "../../api";
import { CodeEditor } from "../CodeEditor";
import { Icon, ICON } from "../Icon";
import { Markdown } from "../Markdown";
import { parseChart } from "../Chart";
import { ArtifactHeading } from "./ArtifactHeading";
import { ArtifactPreview } from "./ArtifactPreview";

export function ArtifactViewer({
  sessionId,
  artifact,
  content,
  onReload,
  onOpenEntry,
}: {
  sessionId: string;
  artifact: ArtifactInfo;
  content: ArtifactContent | null;
  onReload: () => Promise<void>;
  // Folder listings: open a child entry in the viewer (files and subfolders alike).
  onOpenEntry?: (path: string) => void;
}) {
  const [reloadKey, setReloadKey] = useState(0);
  /** The folder this file sits in, or "" at the top of the workspace. */
  const dir = artifact.path.includes("/") ? artifact.path.slice(0, artifact.path.lastIndexOf("/")) : "";
  const isHtml = content?.kind === "html" && !content.error;
  // A chart only if the spec actually parses. A malformed one falls through to the code
  // view, which shows the model what it wrote — more use than an error box.
  const chartSpec = useMemo(
    () => (content?.kind === "chart" && !content.error ? parseChart(content.content || "") : null),
    [content],
  );
  // Best viewed in a real app: spreadsheets, PDFs, and Office docs (pptx/docx can't preview inline)
  const isApp = content?.kind === "sheet" || content?.kind === "pdf" || content?.kind === "office";

  // Editing is what makes the panel a workspace rather than a display: the agent
  // writes a file, the user corrects it in place, and the next turn sees the
  // correction. Only text-backed kinds — a .xlsx edited as text is a corrupt .xlsx,
  // which is why the server refuses those too.
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");
  const editable =
    !!content &&
    !content.error &&
    typeof content.content === "string" &&
    ["markdown", "text", "code", "csv", "html"].includes(content.kind);

  /** Formats whose source is worth seeing rendered as you type. */
  const livePreview: "markdown" | "html" | null =
    editing && (content?.kind === "markdown" || content?.kind === "html")
      ? (content.kind as "markdown" | "html")
      : null;
  const [debouncedDraft, setDebouncedDraft] = useState("");
  useEffect(() => {
    if (!livePreview) return;
    const t = setTimeout(() => setDebouncedDraft(draft), 250);
    return () => clearTimeout(t);
  }, [draft, livePreview]);

  // Switching files must not carry an unsaved draft onto the next one — that would
  // silently save one file's text into another.
  useEffect(() => {
    setEditing(false);
    setDraft("");
    setSaveError("");
  }, [artifact.path]);

  const startEdit = () => {
    setDraft(content?.content || "");
    setSaveError("");
    setEditing(true);
  };

  const save = async () => {
    setSaving(true);
    setSaveError("");
    const res = await writeArtifact(sessionId, artifact.path, draft);
    setSaving(false);
    if (!res.ok) {
      // Kept open with the draft intact — closing the editor on a failed save is
      // how people lose work.
      setSaveError(res.error || "couldn't save");
      return;
    }
    setEditing(false);
    await onReload();
    setReloadKey((k) => k + 1);
  };

  return (
    <div className="artifact-viewer">
      <div className="artifact-head">
        <ArtifactHeading artifact={artifact} dir={dir} />
        <div className="rail-actions">
          {isHtml && (
            <button
              className="artifact-icon-btn"
              onClick={async () => {
                await onReload();
                setReloadKey((k) => k + 1);
              }}
              aria-label="Reload preview"
              title="Reload"
            >
              <Icon name="refresh" size={ICON.md} />
            </button>
          )}
          {editable && !editing && (
            <button className="artifact-icon-btn" onClick={startEdit} aria-label="Edit" title="Edit this file">
              <Icon name="pencil" size={ICON.md} />
            </button>
          )}
          {editing && (
            <>
              <button className="artifact-text-btn primary" onClick={save} disabled={saving} title="Save to disk">
                {saving ? "Saving…" : "Save"}
              </button>
              <button
                className="artifact-text-btn"
                onClick={() => setEditing(false)}
                disabled={saving}
                title="Discard changes"
              >
                Cancel
              </button>
            </>
          )}
          {isApp && (
            <button
              className="artifact-icon-btn"
              onClick={() => revealArtifact(sessionId, artifact.path, "open")}
              aria-label="Open in default app"
              title="Open in default app"
            >
              <Icon name="panelOpen" size={ICON.md} />
            </button>
          )}
          {/* Copy the ABSOLUTE path — the workspace-relative one is useless outside the app
              (tester catch 2026-07-12: it copied just "slack-connector-debug.md"). */}
          <button
            className="artifact-icon-btn"
            onClick={() => navigator.clipboard?.writeText(artifact.abs_path || artifact.path)}
            aria-label="Copy path"
            title="Copy full path"
          >
            <Icon name="copy" size={ICON.md} />
          </button>
          <button
            className="artifact-icon-btn"
            onClick={() => revealArtifact(sessionId, artifact.path, "reveal")}
            aria-label="Show in folder"
            title="Show in folder"
          >
            <Icon name="folder" size={ICON.md} />
          </button>
        </div>
      </div>
      <div className="artifact-preview">
        {editing ? (
          <div className={"artifact-editor" + (livePreview ? " split" : "")}>
            {saveError && <div className="rail-error artifact-save-error">{saveError}</div>}
            <CodeEditor value={draft} path={artifact.path} onChange={setDraft} />
            {/* Editing markdown blind is editing a language you can't see. The preview
                renders the draft rather than the saved file, so the result is visible
                before deciding to keep it — and it is debounced, because re-rendering
                a document (or reloading an iframe) on every keystroke is jarring and,
                for HTML, actually re-runs the page. */}
            {livePreview === "markdown" && (
              <div className="artifact-preview-pane artifact-md">
                <Markdown text={debouncedDraft} />
              </div>
            )}
            {livePreview === "html" && (
              <iframe
                sandbox="allow-scripts allow-same-origin"
                className="artifact-preview-pane artifact-frame"
                srcDoc={debouncedDraft}
              />
            )}
          </div>
        ) : (
          <ArtifactPreview sessionId={sessionId} artifact={artifact} content={content} chartSpec={chartSpec} reloadKey={reloadKey} onOpenEntry={onOpenEntry} />
        )}
      </div>
    </div>
  );
}
