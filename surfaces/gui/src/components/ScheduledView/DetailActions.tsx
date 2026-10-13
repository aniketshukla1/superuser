import { Icon, ICON } from "../Icon";

/** The detail's actions: Save/cancel while editing, the delete confirmation, else
 * Run now, Edit and a set-apart Delete. */
export function DetailActions({
  title,
  editing,
  saving,
  canSave,
  onSave,
  onCancelEdit,
  confirmDelete,
  setConfirmDelete,
  onRemove,
  onRunNow,
  onEdit,
}: {
  title: string;
  editing: boolean;
  saving: boolean;
  canSave: boolean;
  onSave: () => void;
  onCancelEdit: () => void;
  confirmDelete: boolean;
  setConfirmDelete: (on: boolean) => void;
  onRemove: () => void;
  onRunNow: () => void;
  onEdit: () => void;
}) {
  return (
    <div className="sched-actions">
      {editing ? (
        <>
          <button className="btn-primary sm" disabled={saving || !canSave} onClick={onSave}>
            {saving ? "Saving…" : "Save"}
          </button>
          <button className="link" onClick={onCancelEdit}>cancel</button>
        </>
      ) : confirmDelete ? (
        <span className="flex items-center gap-2 text-sm text-muted" role="group" aria-label="Confirm delete">
          Delete “{title}”? Its runs stay in your history.
          <button className="btn sm danger" data-testid="task-delete-confirm" onClick={onRemove}>
            Delete
          </button>
          <button className="btn sm ghost" onClick={() => setConfirmDelete(false)}>
            Cancel
          </button>
        </span>
      ) : (
        <>
          <button className="btn-primary sm" onClick={onRunNow}>
            Run now
          </button>
          <button className="btn sm" onClick={onEdit}>Edit</button>
          {/* Set apart from Edit: the destructive action should not be the next one over. */}
          <button className="btn sm danger-btn ml-3" onClick={() => setConfirmDelete(true)}>
            <Icon name="trash" size={ICON.md} /> Delete…
          </button>
        </>
      )}
    </div>
  );
}
