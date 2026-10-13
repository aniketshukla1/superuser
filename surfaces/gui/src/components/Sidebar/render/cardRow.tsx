import type { SessionInfo } from "../../../types";
import { OriginIcon, ConnectorDot, LiveDot, AttnBadge } from "../Props";
import type { SidebarModel } from "../model/useSidebarModel";
import { rowActions } from "./rowActions";

  // A single-line card row (mock §141 list-flat, subtitle dropped 2026-07-21): title +
  // right-side indicators, with the ⋮ actions kebab revealed on hover. Shared by the flat
  // layout's Pinned and Recent sections. Personas are disabled for the first release; when
  // they return, surface the persona on hover (e.g. in the row tooltip) — not as a subtitle.
  export function cardRow(m: SidebarModel, s: SessionInfo) {
  const { editingId, editValue, setEditingId, setEditValue, rowMenu, props } = m;
    const active = s.session_id === props.activeSession;
    const title = s.title || s.session_id;
    const editing = editingId === s.session_id;
    const commitRename = () => {
      const next = editValue.trim();
      if (next && next !== title) props.onRenameSession(s.session_id, next);
      setEditingId(null);
    };
    return (
      <div
        key={s.session_id}
        className={
          "group list-row relative w-full flex items-center gap-2.5 pl-2 pr-7 rounded-row cursor-pointer text-left " +
          (active
            ? "bg-solid"
            : "hover:bg-paper")
        }
        title={editing ? undefined : s.preview || title}
        onClick={() => {
          if (!editing) props.onSelectSession(s.session_id, s.workspace, s.agent);
        }}
      >
        {/* No leading glyph on session rows (Rohit's call 2026-07-07: the per-session icon
            read as noise in both grouped and chronological). */}
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
            <span className="min-w-0 flex-1 block truncate row-name">
              {title}
            </span>
            <span
              className={
                "flex items-center gap-1.5 shrink-0 group-hover:hidden" +
                (rowMenu?.id === s.session_id ? " hidden" : "")
              }
            >
              <OriginIcon s={s} />
              <ConnectorDot subs={s.subscriptions} />
              <LiveDot state={s.liveness} />
              <AttnBadge n={s.attention || 0} />
            </span>
            {rowActions(m, s, title)}
          </>
        )}
      </div>
    );
  
}
