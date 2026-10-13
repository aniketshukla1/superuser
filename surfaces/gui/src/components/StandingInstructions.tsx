import { useEffect, useState } from "react";
import { getSettings, setStandingInstructions } from "../api";
import { notifyError } from "../notify";

// Standing instructions — what every assistant is told, every task, in your words.
//
// The profile above this learns and fades; AGENTS.md belongs to one
// workspace. This is the one place a person writes down, once, what should always hold
// everywhere. It is sent word for word, after the assistant's own role, with a
// sentence for when the two disagree: the role decides WHAT, these decide HOW.
//
// It guides; it does not enforce. The copy says so and points at Boundaries, because
// "never delete anything" written here is a request a model can misread, and the same
// sentence as a boundary is a refusal it can't.

const DEFAULT_MAX = 4000;

export function StandingInstructions() {
  const [saved, setSaved] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [max, setMax] = useState(DEFAULT_MAX);
  const [busy, setBusy] = useState(false);
  const [flash, setFlash] = useState(false);

  useEffect(() => {
    getSettings()
      .then((s) => {
        setSaved(s.standing_instructions ?? "");
        setText(s.standing_instructions ?? "");
        setMax(s.standing_instructions_max ?? DEFAULT_MAX);
      })
      .catch(() => setSaved(""));
  }, []);

  if (saved === null) return null;
  const count = [...text].length;
  const over = count > max;

  const save = async () => {
    setBusy(true);
    const res = await setStandingInstructions(text).catch(() => null);
    setBusy(false);
    if (!res?.ok) {
      notifyError("Couldn't save your standing instructions.", res?.error);
      return;
    }
    setSaved(res.standing_instructions ?? "");
    setText(res.standing_instructions ?? "");
    setFlash(true);
    setTimeout(() => setFlash(false), 2500);
  };

  return (
    <div data-testid="standing-instructions">
      <div className="jrn-head" style={{ marginTop: 26 }}>
        <div>
          <h3 className="jrn-title">Standing instructions</h3>
          <p className="jrn-sub">
            Told to every assistant at the start of every task, word for word. Where they disagree with an
            assistant&rsquo;s own role, the role decides what it does and these decide how it does it. They guide
            rather than enforce — for anything that must never happen, add a{" "}
            <a className="text-accent" href="#/settings?tab=boundaries">
              boundary
            </a>
            .
          </p>
        </div>
      </div>
      <textarea
        className="w-full px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
        style={{ marginTop: 10 }}
        rows={4}
        placeholder="e.g. Use British spelling. Keep answers under 200 words unless I ask for more. Never push to main."
        data-testid="standing-instructions-text"
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      <div className="jrn-actions" style={{ marginTop: 8 }}>
        <button
          className="jrn-ghost"
          data-testid="standing-instructions-save"
          disabled={busy || over || text === saved}
          onClick={() => void save()}
        >
          {flash ? "Saved" : "Save"}
        </button>
        <span className={"text-xs self-center " + (over ? "text-danger" : "text-faint")}>
          {over ? `${count - max} characters over the ${max} limit` : `${count} / ${max}`}
        </span>
      </div>
    </div>
  );
}
