import { BTN_ACCENT, BTN_BORDERED } from "./INPUT";

/** Save, discard, remove all, and a status line that says which applies. */
export function BoundariesActions({
  dirty,
  saving,
  saved,
  problemCount,
  unreadable,
  hasRules,
  onSave,
  onDiscard,
  onReset,
}: {
  dirty: boolean;
  saving: boolean;
  saved: boolean;
  problemCount: number;
  unreadable: boolean;
  hasRules: boolean;
  onSave: () => void;
  onDiscard: () => void;
  onReset: () => void;
}) {
  return (
    <div className="flex items-center gap-2 flex-wrap">
      <button className={BTN_ACCENT} disabled={!dirty || saving} onClick={onSave} data-testid="boundaries-save">
        {saving ? "Saving…" : "Save"}
      </button>
      <button
        className={BTN_BORDERED}
        disabled={!dirty || saving || unreadable}
        onClick={onDiscard}
      >
        Discard changes
      </button>
      {!unreadable && hasRules && (
        <button className="text-sm text-danger px-2 py-1" onClick={onReset}>
          Remove all boundaries
        </button>
      )}
      <span className="text-sm text-muted" role="status" data-testid="boundaries-status">
        {problemCount > 0
          ? `Not saved — ${problemCount === 1 ? "one rule needs" : `${problemCount} rules need`} fixing.`
          : saved
            ? "Saved. It applies to the next action any assistant takes."
            : dirty
              ? "Unsaved changes."
              : ""}
      </span>
    </div>
  );
}
