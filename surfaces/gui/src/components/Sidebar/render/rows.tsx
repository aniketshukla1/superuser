import type { SessionInfo } from "../../../types";
import { Icon, ICON } from "../../Icon";
import { compactAge, OriginIcon, LiveDot, AttnBadge } from "../Props";
import type { SidebarModel } from "../model/useSidebarModel";
import { rowActions } from "./rowActions";

  // The nested sub-thread rows under a fanned-out session.
  export function threadRows(m: SidebarModel, parent: SessionInfo) {
  const { threadsByParent, props } = m;
  return (threadsByParent.get(parent.session_id) ?? []).map((t) => (
      <button
        key={t.session_id}
        className={
          "w-full flex items-center gap-1.5 pl-5 pr-2 py-1 text-sm text-left rounded-md truncate " +
          (props.activeSession === t.session_id ? "bg-accentSoft text-accent" : "text-muted hover:text-ink")
        }
        title={t.title}
        onClick={() => props.onSelectSession(t.session_id, t.workspace, t.agent)}
      >
        <span className="text-faint shrink-0">└</span>
        <span className="truncate">{t.title || "Thread"}</span>
      </button>
    ));

}

  // A compact session row (mock §141 grouped/recent rows): one-line title + right-side indicators,
  // with the ⋮ actions kebab revealed on hover. Used in accordion bodies + grouped cards.
  export function sessionRow(m: SidebarModel, s: SessionInfo, opts: { showTime?: boolean } = {}) {
  const { editingId, editValue, setEditingId, setEditValue, rowMenu, props } = m;
    const title = s.title || s.session_id;
    const editing = editingId === s.session_id;
    const active = s.session_id === props.activeSession;
    const commitRename = () => {
      const next = editValue.trim();
      if (next && next !== title) props.onRenameSession(s.session_id, next);
      setEditingId(null);
    };
    return (
      <div
        key={s.session_id}
        className={
          // Artifact metrics: 5px..8px padding, 9px gap, 7px radius. Active is --line, the
          // same step the segmented control uses, so "selected" means one thing everywhere
          // in this sidebar rather than a different near-invisible wash per component.
          "group list-row relative flex items-center gap-2 pl-2 pr-7 rounded-row text-left cursor-pointer " +
          (active ? "bg-line" : "hover:bg-solid")
        }
        onClick={() => {
          if (!editing) props.onSelectSession(s.session_id, s.workspace, s.agent);
        }}
        title={editing ? undefined : s.preview || title}
      >
        {editing ? (
          <input
            className="flex-1 min-w-0 px-1.5 py-0.5 rounded-md bg-panel border border-accent row-name text-ink outline-none"
            value={editValue}
            autoFocus
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === "Enter") commitRename();
              else if (e.key === "Escape") setEditingId(null);
            }}
          />
        ) : (
          <>
            {/* The leading pip, on the session you are in and nowhere else — it says
                "here", and a column of hollow rings beside every other row says nothing
                anyone could name. The space is kept so the names stay in one column.
                Not LiveDot: that one is transient state (working / sleeping) and
                right-aligned, and it renders nothing at all for an idle session. */}
            <span
              className={"w-[5px] h-[5px] rounded-full shrink-0 " + (active ? "bg-accent" : "")}
              aria-hidden="true"
            />
            {/* Size comes from `.row-name` in styles.css, next to the rule that sets
                the row height, because the two constrain each other.

                Background only, never weight.
                The active row already carries two marks — the row's own `bg-line`
                against the sidebar's `bg-paper`, a fifteen-value step, and the
                accent dot to its left. Bolding the text on top of those made the
                open chat's name a different weight from the chat itself, which
                reads as an inconsistency long before anyone works out that it is
                the selection state. State belongs to the row; the name is just
                the name. */}
            <span className="min-w-0 flex-1 flex items-center gap-1.5 truncate row-name text-ink">
              {s.pinned && <Icon name="pin" size={ICON.sm} className="text-faint shrink-0" />}
              <span className="truncate">{title}</span>
            </span>
            <span
              className={
                "flex items-center gap-1.5 shrink-0 group-hover:hidden" +
                (rowMenu?.id === s.session_id ? " hidden" : "")
              }
            >
              {opts.showTime && compactAge(s.updated_at) && (
                <span className="text-xs text-faint tabular-nums">{compactAge(s.updated_at)}</span>
              )}
              <OriginIcon s={s} />
              <LiveDot state={s.liveness} />
              <AttnBadge n={s.attention || 0} />
            </span>
            {rowActions(m, s, title)}
          </>
        )}
      </div>
    );
  
}
