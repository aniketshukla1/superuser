import { Fragment } from "react";
import { PersonaGlyph } from "../personaIcon";
import { ICON, Icon } from "../Icon";
import { LiveDot, AttnBadge } from "./Props";
import type { SidebarModel } from "./model/useSidebarModel";
import { pinnedBand } from "./render/bands";
import { scheduledBand } from "./render/bands";
import { recentHeader } from "./render/recentHeader";
import { projectFirstList } from "./render/projectFirstList";
import { surfaceBody } from "./render/surfaceBody";
import { sessionRow } from "./render/rows";
import { cardRow } from "./render/cardRow";
import { threadRows } from "./render/rows";

/** The scrolling list: pinned, scheduled, and the sessions for the half on show. */
export function SidebarList({ m }: { m: SidebarModel }) {
  const { hasBothKinds, surfaceTab, layout, shownSurfaces, isExpanded, isCurrent, onHeaderClick,
    liveByPersona, attnByPersona, openSession, recentSessions, normalizedQuery, recentExpanded,
    RECENT_PEEK, setRecentExpanded, props } = m;
  return (
    <div className="flex-1 overflow-y-auto px-2 pt-4 pb-2">
        <div className="space-y-3">
          {pinnedBand(m)}
          {scheduledBand(m)}
          <div>
            {/* "Recent" labels a chronological list — which is what Chat shows. Under Code the
                list is projects, in the order YOU work in them, so the label was describing
                something that wasn't there and design/Main.dc.html has no such heading. The
                group/filter control it carried is a Chat affordance too (it flips the persona
                accordion), so both go together rather than leaving a stray icon. */}
            {!(hasBothKinds && surfaceTab === "code") && recentHeader(m)}
            {/* Under Code the folder is what you navigate by, so projects are top
                level and the persona accordion falls away entirely. Chat keeps the
                persona list — those sessions have no folder to group by. Only when
                the Chat/Code control is actually offered: with one kind of persona
                there is no tab to be on, and the old grouping is still correct. */}
            {hasBothKinds && surfaceTab === "code" ? (
              projectFirstList(m)
            ) : layout === "grouped" ? (
            <div className="space-y-1.5">
              {shownSurfaces.map((s, si) => {
                const expanded = isExpanded(s.key);
                const kind = (s as any).kind === "workbench" ? "workbench" : "assistant";
                const prevKind = si > 0 ? ((shownSurfaces[si - 1] as any).kind === "workbench" ? "workbench" : "assistant") : null;
                // Only label when both kinds are present — a user with no workbenches
                // shouldn't see a distinction that doesn't apply to them. With the
                // Chat/Code control filtering to one kind, this is false by
                // construction and the headings disappear: the control already says
                // which half you are looking at, so a heading repeating it is noise.
                const mixed = shownSurfaces.some((x) => (x as any).kind === "workbench")
                  && shownSurfaces.some((x) => (x as any).kind !== "workbench");
                const header = mixed && kind !== prevKind
                  ? <div className="px-2 pt-1.5 pb-1 text-xs r text-faint" key={`h-${kind}`}>
                      {kind === "workbench" ? "Workbenches" : "Assistants"}
                    </div>
                  : null;
                return (
                  // When expanded, the wrapper carries the recessed fill so the header sits INSIDE
                  // the block with its sessions (one connected group). Collapsed = a plain row.
                  <Fragment key={s.key}>
                  {header}
                  <div
                    className={expanded ? "rounded-xl bg-paper/70 overflow-hidden" : ""}
                  >
                    <div
                      className={
                        "nav-row flex items-center gap-2.5 px-2 cursor-pointer select-none " +
                        (expanded
                          ? ""
                          : isCurrent(s.key)
                            ? "rounded-lg bg-paper"
                            : "rounded-lg hover:bg-paper")
                      }
                      onClick={() => onHeaderClick(s.key)}
                    >
                      {/* The persona's own mark. `surfaceFromPersona` has always computed
                          one and nothing drew it, so every assistant looked like every
                          other assistant — a column of identical text rows. */}
                      <PersonaGlyph icon={(s as any).icon} family={(s as any).family} size={ICON.md} />
                      {/* Dressed like a project name in Workspace, because it is the same
                          kind of thing: the label on a group of sessions. It used to be
                          13px ink, bolder again for the active one, which made a group
                          header heavier than the session titles inside it — and the note
                          on the project rows already worked out why that is wrong:
                          brightening the label makes it compete with its own contents,
                          and which group you are in is obvious from the session that is
                          lit inside it. Two tabs, one treatment. */}
                      <span className="min-w-0 flex-1 truncate text-sm text-muted">
                        {s.label}
                      </span>
                      <LiveDot state={liveByPersona.get(s.key)} />
                      <AttnBadge n={attnByPersona.get(s.key) || 0} />
                      {/* Start a session in THIS persona, from the row that names it.
                          The top split button can already do it behind a ▾, which is a
                          menu you have to open to find the thing you are looking at —
                          and a "+" that only appears on hover is the same problem in a
                          smaller box. Rendered at rest, like the project rows'. */}
                      <button
                        className="nav-icon-btn nav-row-plus"
                        title={`New session with ${s.label}`}
                        aria-label={`New session with ${s.label}`}
                        data-testid={`persona-new-${s.key}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          props.onNewSession(s.key);
                        }}
                      >
                        <Icon name="plus" size={ICON.md} />
                      </button>
                      {/* Persona configuration moved to Settings ▸ Personas (Rohit's call
                          2026-07-07) — the per-group gear read as clutter here. */}
                      <Icon
                        name={expanded ? "chevronDown" : "chevronRight"}
                        size={ICON.md}
                        className="text-faint shrink-0"
                      />
                    </div>
                    {expanded && surfaceBody(m)}
                    {/* Collapsed, but it holds the conversation you are in: that one row stays.
                        Opening another assistant folded this group and took the open chat off
                        the screen with it — the list denying what the chat area was showing. */}
                    {!expanded && isCurrent(s.key) && openSession && (
                      <div className="px-1.5 pb-1.5">{sessionRow(m, openSession)}</div>
                    )}
                  </div>
                  </Fragment>
                );
              })}
            </div>
            ) : (
            <div className="space-y-0.5">
              {recentSessions.length === 0 ? (
                <div className="px-2 py-1.5 text-sm text-faint leading-snug">
                  {normalizedQuery ? "No matching conversations." : "No conversations yet."}
                </div>
              ) : (
                <>
                  {(recentExpanded
                    ? recentSessions
                    : recentSessions.slice(0, RECENT_PEEK)
                  ).map((s) => (
                    <div key={s.session_id}>
                      {cardRow(m, s)}
                      {threadRows(m, s)}
                    </div>
                  ))}
                  {recentSessions.length > RECENT_PEEK && (
                    <button
                      className="w-full text-left px-2 py-1.5 text-sm text-muted hover:text-ink"
                      onClick={() => setRecentExpanded((v) => !v)}
                    >
                      {recentExpanded
                        ? "Show less"
                        : `Show ${recentSessions.length - RECENT_PEEK} more`}
                    </button>
                  )}
                </>
              )}
            </div>
            )}
          </div>
        </div>
      </div>
  );
}
