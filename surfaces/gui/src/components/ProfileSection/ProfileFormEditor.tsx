import type { ProfileForm, ProfileState } from "../../api";
import { FIELDS } from "./CLASSES";

/** "What you've told it": the profile form and its one Save action. */
export function ProfileFormEditor({
  state,
  form,
  setForm,
  busy,
  saved,
  onSave,
}: {
  state: ProfileState;
  form: ProfileForm;
  setForm: (f: ProfileForm) => void;
  busy: boolean;
  saved: boolean;
  onSave: () => void;
}) {
  return (
    <>
      <div className="jrn-head" style={{ marginTop: 26 }}>
        <div>
          <h3 className="jrn-title">What you've told it</h3>
          <p className="jrn-sub">
            These never fade and never have to prove themselves — but telling it something
            different in conversation will override one, so you don't have to come back here to
            correct it. Clearing a field removes it immediately.
          </p>
        </div>
      </div>
      <div className="grid gap-3" style={{ marginTop: 10 }}>
        {FIELDS.map(({ key, label, hint, area }) => (
          <label key={key} className="block">
            <div className="text-sm font-medium text-ink">{label}</div>
            {hint && <div className="text-xs text-faint mb-1">{hint}</div>}
            {area ? (
              <textarea
                className="w-full max-w-2xl px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
                rows={2}
                data-testid={`profile-field-${key}`}
                value={form[key]}
                onChange={(e) => setForm({ ...form, [key]: e.target.value })}
              />
            ) : (
              <input
                className="w-full max-w-md px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
                data-testid={`profile-field-${key}`}
                value={form[key]}
                onChange={(e) => setForm({ ...form, [key]: e.target.value })}
              />
            )}
          </label>
        ))}
      </div>
      {/* The form's one action, styled as one, and live only when there is something to save.
          It was a ghost button under nine fields, identical to Pin and Forget, and pressable
          with nothing changed. The path is the file's name, with the whole path on hover. */}
      <div className="jrn-actions mt-3">
        <button
          className="btn-primary sm"
          data-testid="profile-save"
          disabled={busy || JSON.stringify(form) === JSON.stringify(state.form)}
          onClick={onSave}
        >
          {saved ? "Saved" : "Save changes"}
        </button>
        {state.path && (
          <span className="text-xs text-faint self-center" title={state.path}>
            Stored in {state.path.split("/").pop()}
          </span>
        )}
      </div>
    </>
  );
}
