import { useRef, useState } from "react";
import { createPersona, PERSONAS_CHANGED } from "../../api";
import { DEFAULT_TOOLS, ICONS, MODES, TEXTY, TOOLS, type Picked } from "./TOOLS";

export function CreateAssistantModal({ onClose, onCreated }: { onClose: () => void; onCreated?: (id: string) => void }) {
  const [name, setName] = useState("");
  const [duties, setDuties] = useState("");
  const [icon, setIcon] = useState<string>("sparkles");
  const [mode, setMode] = useState("interactive");
  const [tools, setTools] = useState<string[]>(DEFAULT_TOOLS);
  const [starters, setStarters] = useState<string[]>(["", "", ""]);
  const [files, setFiles] = useState<Picked[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  const toggleTool = (id: string) =>
    setTools((t) => (t.includes(id) ? t.filter((x) => x !== id) : [...t, id]));

  const addFiles = async (list: FileList | null) => {
    if (!list?.length) return;
    const accepted: Picked[] = [];
    const skipped: string[] = [];
    for (const f of Array.from(list)) {
      if (!TEXTY.test(f.name)) {
        skipped.push(f.name);
        continue;
      }
      try {
        accepted.push({ name: f.name, content: await f.text() });
      } catch {
        skipped.push(f.name);
      }
    }
    if (accepted.length) setFiles((prev) => [...prev, ...accepted]);
    setNote(
      skipped.length
        ? `Skipped ${skipped.length} file${skipped.length > 1 ? "s" : ""} — text files only for now (${skipped.slice(0, 2).join(", ")}).`
        : null,
    );
  };

  const submit = async () => {
    if (!name.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      const res = await createPersona({
        name: name.trim(),
        responsibilities: duties.trim(),
        icon,
        starters: starters.map((s) => s.trim()).filter(Boolean),
        permission_mode: mode,
        tools,
        files,
      });
      if (!res.ok) {
        setError(res.error || "Couldn't create that assistant.");
        return;
      }
      window.dispatchEvent(new Event(PERSONAS_CHANGED));
      onCreated?.(res.persona?.id || "");
      onClose();
    } catch (e) {
      setError(String((e as Error).message || e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="ca-scrim" onClick={onClose}>
      <div className="ca-modal" onClick={(e) => e.stopPropagation()} role="dialog" aria-label="New assistant">
        <div className="ca-title">New assistant</div>

        <div className="ca-body">
          <label className="ca-label" htmlFor="ca-name">Name</label>
          <input
            id="ca-name"
            className="ca-input"
            autoFocus
            placeholder="e.g. Finance"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />

          <label className="ca-label">Icon</label>
          <div className="ca-chips">
            {ICONS.map((i) => (
              <button key={i} type="button" className={"ca-chip" + (icon === i ? " is-on" : "")} onClick={() => setIcon(i)}>
                {i}
              </button>
            ))}
          </div>

          <label className="ca-label" htmlFor="ca-duties">
            What is it responsible for? <span className="ca-hint">(becomes how it introduces itself)</span>
          </label>
          <textarea
            id="ca-duties"
            className="ca-input ca-textarea"
            rows={3}
            placeholder="e.g. Keep an eye on budgets and cash flow, and turn receipts into a spreadsheet."
            value={duties}
            onChange={(e) => setDuties(e.target.value)}
          />

          <label className="ca-label">
            Opening prompts <span className="ca-hint">(optional — shown as one-click chips)</span>
          </label>
          {starters.map((v, i) => (
            <input
              key={i}
              className="ca-input ca-starter"
              placeholder={i === 0 ? "e.g. Build me a budget" : "…"}
              value={v}
              onChange={(e) => setStarters((s) => s.map((x, j) => (j === i ? e.target.value : x)))}
            />
          ))}

          <label className="ca-label">How much rope?</label>
          <div className="ca-modes">
            {MODES.map((m) => (
              <button
                key={m.id}
                type="button"
                className={"ca-mode" + (mode === m.id ? " is-on" : "")}
                onClick={() => setMode(m.id)}
                title={m.hint}
              >
                <span className="ca-mode-label">{m.label}</span>
                <span className="ca-mode-hint">{m.hint}</span>
              </button>
            ))}
          </div>

          <label className="ca-label">Tools it can use</label>
          <div className="ca-chips">
            {TOOLS.map((t) => (
              <button
                key={t.id}
                type="button"
                className={"ca-chip" + (tools.includes(t.id) ? " is-on" : "")}
                onClick={() => toggleTool(t.id)}
                title={t.hint}
              >
                {t.label}
              </button>
            ))}
          </div>

          <label className="ca-label">
            Reference files <span className="ca-hint">(text files it should know about)</span>
          </label>
          <div
            className="ca-drop"
            onClick={() => fileInput.current?.click()}
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => {
              e.preventDefault();
              addFiles(e.dataTransfer.files);
            }}
          >
            + Drop files here or click to browse
          </div>
          <input
            ref={fileInput}
            type="file"
            multiple
            hidden
            onChange={(e) => {
              addFiles(e.target.files);
              e.target.value = "";
            }}
          />
          {files.length > 0 && (
            <div className="ca-files">
              {files.map((f, i) => (
                <span className="ca-file" key={f.name + i}>
                  {f.name}
                  <button type="button" onClick={() => setFiles((p) => p.filter((_, j) => j !== i))} aria-label={`Remove ${f.name}`}>
                    ×
                  </button>
                </span>
              ))}
            </div>
          )}
          {note && <div className="ca-note">{note}</div>}
          {error && <div className="ca-error">{error}</div>}
        </div>

        <div className="ca-actions">
          <button className="ca-btn" onClick={onClose} type="button">Cancel</button>
          <button className="ca-btn is-primary" onClick={submit} disabled={!name.trim() || busy} type="button">
            {busy ? "Creating…" : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}
