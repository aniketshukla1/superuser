import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";
import { FilesButton } from "./FilesButton";
import { TerminalButton } from "./TerminalButton";
import { BrowserButton } from "./BrowserButton";
import { SessionMenu } from "./SessionMenu";

/** The right of the top bar: tool buttons and the session menu. */
export function TopbarActions({ m }: { m: AppModel }) {
  const { beginWindowDrag, railChosen, setRailHidden, railHidden } = m;
  return (
    <div className="main-topbar-side main-topbar-actions" onPointerDown={beginWindowDrag}>
            {/* Shown whenever this conversation has produced files, like Terminal and
                Browser beside it — and a toggle, with an `is-on` state, for the same
                reason they have one.

                It used to be gated on `railHidden`, which made it two different bugs
                wearing one condition. It vanished the moment you pressed it, because
                pressing it opens the rail. And it never appeared at all in a window
                wider than 1040px, where the rail starts OPEN — so on the desktop app
                it did not exist, while a narrow browser pane showed it. A control
                whose visibility depends on the state it is about to change can only
                be right before you use it. */}
            <FilesButton m={m} />
            {/* Terminal and Browser are tools you reach for, not places the panel is — so
                they are buttons here rather than two permanent tabs standing empty in the
                strip you scan to find what the turn produced. Clicking one opens the panel
                on it.

                Shown per CAPABILITY, and the two capabilities are different ones. Both
                used to hang off `hasShell`, which meant a knowledge assistant holding
                `web` could call `browser_open`, load a page into the panel, and leave
                the user with no button to go and look at it. See `personaScope`. */}
            <TerminalButton m={m} />
            <BrowserButton m={m} />
            {/* Session actions. The sidebar row's hover menu has carried these all along, but
                that is a menu you have to go and find on a row you may have scrolled past —
                and the session you want to act on is almost always the one you are reading. */}
            <SessionMenu m={m} />
            {/* §32: the panel toggle is the ONE session-panel entry, for every persona
                (the rail now carries Access, so code-family gets it too). It was written
                as `agent !== "chat"` when a shell-less "chat" persona existed; that
                persona is gone, so the exclusion excluded nobody and is now simply not
                written. */}
            {(
              <button
                className="topbar-icon-btn"
                onMouseDown={(e) => e.stopPropagation()}
                onClick={() => {
                  railChosen.current = true;
                  setRailHidden((h) => !h);
                }}
                aria-label={railHidden ? "Show side panel" : "Hide side panel"}
                data-tip={railHidden ? "Show side panel" : "Hide side panel"}
              >
                <Icon name="sidebarRight" size={ICON.md} />
              </button>
            )}
          </div>
  );
}
