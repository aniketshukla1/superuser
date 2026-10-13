import type { SessionInfo } from "../../../types";
import { type IconName, Icon, ICON } from "../../Icon";
import type { SidebarModel } from "../model/useSidebarModel";

  // Row actions live behind ONE ⋮ kebab per row (FB-011: four hover icons read as clutter) —
  // the menu offers Rename · Pin/Unpin · Archive/Unarchive · Delete, with the two-step delete
  // confirm kept inside it. Shared by BOTH row styles, so the chronological cardRow offers the
  // same actions as the persona accordion's sessionRow (owner ask 2026-07-09).
  export function rowActions(m: SidebarModel, s: SessionInfo, title: string) {
  const { rowMenu, closeRowMenu, openRowMenu, setEditingId, setEditValue, setDeleting, props } = m;
    const menuOpen = rowMenu?.id === s.session_id;
    const item = (testid: string, icon: IconName, label: string, onClick: () => void) => (
      <button
        className="w-full flex items-center gap-2 px-2.5 py-1.5 text-md text-left hover:bg-paper"
        data-testid={testid}
        role="menuitem"
        onClick={() => {
          closeRowMenu();
          onClick();
        }}
      >
        <Icon name={icon} size={ICON.md} className="shrink-0 text-muted" />
        <span className="flex-1">{label}</span>
      </button>
    );
    return (
      <span
        // Stay visible while this row's menu is open — the pointer may be on the menu, off the row.
        //
        // `hidden` is `display: none`, and nothing display:none is focusable — which
        // made Rename / Pin / Archive / Delete mouse-only, with no other route to them
        // anywhere in the app (UX-REPORT §7). `group-focus-within` reveals the row's
        // actions when a keyboard user tabs into the row, and `focus-within` on this
        // span keeps them up once focus is on the kebab itself.
        className={
          // Visible at rest, quietly. It was `opacity-0` until hover, which was the
          // right call when four icons sat here (FB-011) and the wrong one now that
          // it is a single glyph: an action you cannot see is an action that does not
          // exist, and "the three dots don't work" is what that looks like from
          // outside. It still brightens on hover, and still comes fully forward when
          // the row is hovered or focused.
          (menuOpen
            ? "flex opacity-100"
            : "opacity-45 group-hover:opacity-100 group-focus-within:opacity-100 focus-within:opacity-100 flex") +
          // Centred by flex, NOT by `top-1/2 -translate-y-1/2`.
          //
          // That translate is why the menu "didn't work on any chat". A transformed
          // element becomes the containing block for `position: fixed` descendants,
          // and the menu is fixed at coordinates taken from getBoundingClientRect —
          // viewport coordinates. So every menu opened offset by however far down
          // the sidebar its own row happened to be: sometimes visible, usually not,
          // never where you clicked. The button worked the whole time; what it
          // opened landed somewhere you could not see.
          //
          // `inset-y-0` + `items-center` centres the same way and leaves the span
          // untransformed, so `fixed` means fixed to the window again.
          " items-center shrink-0 absolute right-1 inset-y-0"
        }
        onClick={(e) => e.stopPropagation()}
      >
        <button
          title="Session actions"
          // Named per row: six buttons all called "Session actions" are six identical
          // announcements with nothing to tell them apart.
          aria-label={`Session actions for ${title}`}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          data-testid="row-menu"
          className={
            // 24px: rows sit close enough vertically that the WCAG 2.5.8 spacing
            // exception doesn't rescue a 20px target (UX-REPORT §8).
            "w-6 h-6 grid place-items-center rounded hover:bg-paper " +
            (menuOpen ? "text-ink bg-paper" : "text-faint hover:text-ink")
          }
          onClick={(e) => (menuOpen ? closeRowMenu() : openRowMenu(s.session_id, e.currentTarget))}
        >
          {/* Vertical kebab = the horizontal glyph rotated — no extra icon needed. */}
          <Icon name="moreHorizontal" size={ICON.md} className="rotate-90" />
        </button>
        {menuOpen && (
          <>
            <div className="fixed inset-0 z-40" onClick={closeRowMenu} />
            <div
              className="fixed z-50 w-40 rounded-xl border border-line bg-panel shadow-xl py-1"
              style={{ top: rowMenu!.top, left: rowMenu!.left }}
              role="menu"
            >
              {item("row-menu-rename", "pencil", "Rename", () => {
                setEditingId(s.session_id);
                setEditValue(title);
              })}
              {item("row-menu-pin", "pin", s.pinned ? "Unpin" : "Pin", () =>
                props.onTogglePin(s.session_id, !s.pinned),
              )}
              {item("row-menu-archive", "archive", s.archived ? "Unarchive" : "Archive", () =>
                props.onArchiveSession(s.session_id, !s.archived),
              )}
              <div className="h-px bg-line my-1 mx-2" />
              {(
                // Opens a dialog rather than arming the item in place. The old
                // two-step changed "Delete" to "Delete?" in the same red text —
                // one character apart, for something irreversible — so the pause
                // read as a click that had not registered and people clicked
                // again without ever knowing they had been asked. And what goes
                // is more than the transcript: the session folder holds every
                // file the agent produced, which a menu row has no room to say.
                <button
                  className="w-full flex items-center gap-2 px-2.5 py-1.5 text-md text-left text-danger hover:bg-paper"
                  data-testid="row-menu-delete"
                  role="menuitem"
                  onClick={() => {
                    closeRowMenu();
                    setDeleting({ id: s.session_id, title, messages: s.messages ?? 0 });
                  }}
                >
                  <Icon name="trash" size={ICON.md} className="shrink-0" />
                  <span className="flex-1">Delete…</span>
                </button>
              )}
            </div>
          </>
        )}
      </span>
    );
  
}
