import { useEffect, useState } from "react";
import { readArtifact, writeArtifact } from "../../api";
import { Markdown } from "../Markdown";
import { AGENTS, Section } from "./GuidedAdd";

/** The assistant's standing instructions: AGENTS.md in this workspace, rendered, editable. */
export function InstructionsSection({ sessionId }: { sessionId: string }) {
  const [instructions, setInstructions] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState("");

  // AGENTS.md may not exist yet; "not there" is an empty instructions block, not an
  // error — the user writing the first line is how it gets created.
  useEffect(() => {
    let dead = false;
    readArtifact(sessionId, AGENTS)
      .then((c) => !dead && setInstructions(c.ok ? c.content || "" : ""))
      .catch(() => !dead && setInstructions(""));
    setEditing(false);
    return () => {
      dead = true;
    };
  }, [sessionId]);

  const saveInstructions = async () => {
    setSaving(true);
    setSaveError("");
    const res = await writeArtifact(sessionId, AGENTS, draft);
    setSaving(false);
    if (!res.ok) {
      setSaveError(res.error || "couldn't save");
      return;
    }
    setInstructions(draft);
    setEditing(false);
  };

  return (
    <Section
      title="Instructions"
      action={editing ? undefined : instructions ? "Edit" : "Add"}
      onAction={() => {
        setDraft(instructions || "");
        setEditing(true);
      }}
    >
      <div className="asst-hint">
        Kept in <code>{AGENTS}</code> in this workspace, and read at the start of every turn.
      </div>
      {editing ? (
        <div className="asst-edit">
          {saveError && <div className="rail-error">{saveError}</div>}
          <textarea
            className="asst-textarea"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="How should this assistant work? Anything here applies to every conversation in this workspace."
            spellCheck={false}
            autoFocus
          />
          <div className="asst-guided-foot">
            <button className="artifact-text-btn primary" onClick={saveInstructions} disabled={saving}>
              {saving ? "Saving…" : "Save"}
            </button>
            <button className="artifact-text-btn" onClick={() => setEditing(false)} disabled={saving}>
              Cancel
            </button>
          </div>
        </div>
      ) : instructions ? (
        // Rendered, not shown as source. These are the user's own words about how
        // they want to be worked with — reading them as raw markdown makes the one
        // human thing on this tab look like a config file.
        <div className="asst-instructions">
          <Markdown text={instructions} />
        </div>
      ) : (
        <div className="rail-muted">No standing instructions yet.</div>
      )}
    </Section>
  );
}
