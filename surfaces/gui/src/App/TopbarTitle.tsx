import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** The left of the top bar: nav controls and the editable chat title. */
export function TopbarTitle({ m }: { m: AppModel }) {
  const { beginWindowDrag, navCollapsed, toggleNav, startNewSession, setSearchOpen, renamingTitle,
      setRenamingTitle, commitTitle, activeInfo, activeTitle, hasHistory, subtitleShown } = m;
  return (
    <div className="main-topbar-side" onPointerDown={beginWindowDrag}>
            {navCollapsed && (
              <div
                className="flex items-center gap-1"
                data-testid="topbar-cluster"
                onPointerDown={(e) => e.stopPropagation()}
              >
                <button
                  className="topbar-icon-btn"
                  onClick={toggleNav}
                  aria-label="Show sidebar"
                  data-tip="Show sidebar (⌘B)"
                >
                  <Icon name="sidebar" size={ICON.md} />
                </button>
                <button
                  className="topbar-icon-btn"
                  onClick={() => startNewSession()}
                  aria-label="New session"
                  data-tip="New session"
                >
                  <Icon name="plus" size={ICON.md} />
                </button>
                <button
                  className="topbar-icon-btn"
                  onClick={() => setSearchOpen(true)}
                  aria-label="Search"
                  data-tip="Search"
                >
                  <Icon name="search" size={ICON.md} />
                </button>
              </div>
            )}
            {/* §32: no session-settings row up here anymore — the §23 rest/hover/click glance
                machinery retired with the drawer. "What can this touch" lives permanently on
                the rail's Access section header; the panel toggle is the one entry. */}
            {/* Title + facts, LEFT-aligned and on one line — design/Main.dc.html has the
                session name and its model as a row, not a stacked centred block. It lives
                inside the left region because as a sibling of two flex:1 regions it was
                centred by construction, whichever way its own alignment pointed.

                The facts are a chip rather than a subtitle: on one line a plain grey string
                after a bold title reads as part of the title. */}
            <div className="main-title" onPointerDown={beginWindowDrag}>
              <h1 className="main-title-h">
              {renamingTitle !== null ? (
                <input
                  className="main-title-input"
                  aria-label="Conversation name"
                  data-testid="title-input"
                  autoFocus
                  value={renamingTitle}
                  placeholder="Name this conversation"
                  onPointerDown={(e) => e.stopPropagation()}
                  onChange={(e) => setRenamingTitle(e.target.value)}
                  onBlur={commitTitle}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") commitTitle();
                    else if (e.key === "Escape") setRenamingTitle(null);
                  }}
                />
              ) : activeInfo ? (
                // The name is editable where it is read. `stopPropagation` so a click names the
                // conversation rather than starting a window drag.
                <button
                  className="main-title-text is-editable"
                  data-testid="title-button"
                  aria-label={`Rename “${activeTitle}”`}
                  data-tip={`${activeTitle} — click to rename`}
                  onPointerDown={(e) => e.stopPropagation()}
                  onClick={() => setRenamingTitle(activeInfo.named ? activeInfo.title ?? "" : "")}
                >
                  {activeTitle}
                </button>
              ) : (
                <span className="main-title-text title-ghost" data-tip={activeTitle}>
                  {activeTitle}
                </span>
              )}
              </h1>
              {/* Plain facts, no affordance: the persona page it used to open is hidden for
                  this release (owner ask 2026-07-22). The assistant's name is dropped when the
                  heading above already IS the assistant's name — the header read "Main · Main". */}
              {hasHistory && subtitleShown.length > 0 && (
                <span className="title-sub" data-testid="session-subtitle">
                  {subtitleShown.join(" · ")}
                </span>
              )}
            </div>
          </div>
  );
}
