import type { SkillRow } from "../../api";
import { Icon, ICON } from "../Icon";
import { BADGE, BTN_BORDERED } from "./INPUT";

/** One skill: name, where it came from, what it does; delete (asks twice) and on/off. */
export function SkillRowItem({
  row,
  armed,
  onRemove,
  onDisarm,
  onToggle,
}: {
  row: SkillRow;
  armed: boolean;
  onRemove: () => void;
  onDisarm: () => void;
  onToggle: (on: boolean) => void;
}) {
  return (
    <div className="flex items-center gap-3 px-4 py-3">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className={`text-md font-medium ${row.enabled ? "" : "text-muted"}`}>
            {row.name}
          </span>
          {row.source !== "local" ? <span className={BADGE}>{row.source}</span> : null}
        </div>
        {/* Full description, wrapping — a skill's one-liner is its menu entry; cutting
            it mid-word hid what the skill does (live drive). */}
        <div className="text-sm text-muted leading-prose">{row.description}</div>
      </div>
      <button
        className={BTN_BORDERED}
        aria-label={`Delete ${row.name}`}
        onClick={onRemove}
        onBlur={onDisarm}
      >
        {armed ? "Confirm delete" : <Icon name="trash" size={ICON.md} />}
      </button>
      <label className="inline-flex items-center gap-1.5 text-sm text-muted">
        <input
          type="checkbox"
          role="switch"
          aria-label={`${row.name} enabled`}
          checked={row.enabled}
          onChange={(e) => onToggle(e.target.checked)}
        />
        On
      </label>
    </div>
  );
}
