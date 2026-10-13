import { NewSessionSplit } from "./NewSessionSplit";
import { Icon, ICON } from "../Icon";
import { AttnBadge, menuKeys } from "./Props";
import type { SidebarModel } from "./model/useSidebarModel";
import { railRows } from "./render/rail";

/** New session and the rail: destinations, and the More disclosure for the rest. */
export function SidebarNav({ m }: { m: SidebarModel }) {
  const { personas, browseKey, setCreatingAssistant, shownRail, moreButtonRef, moreOpen,
    hiddenActive, setMoreOpen, hiddenAttention, moreMenuRef, hiddenRail, navTarget,
    totalAttention, setEditSidebarOpen, props } = m;
  return (
    <nav className="px-2 space-y-px" aria-label="Surfaces">
        {/* New session leads this group rather than sitting above it as a filled button.
            `browseKey`, not `props.agent`. Expanding a persona browses it without switching the
            chat area, and the rule stated where openKey is declared is that "New session" is one
            of the two actions that switches. This button was reading the ACTIVE persona instead,
            so clicking Assistant and then New session opened a Coder session — the click looked
            ignored, because the only visible effect was in a pane you weren't watching.
            It used to be harmless: each expanded persona had its own New-session action, and this
            button was just the shortcut for the current one. That per-persona action was removed
            (see ((...a: Parameters<typeof surfaceBody> extends [unknown, ...infer R] ? R : never) => surfaceBody(m, ...a))) and this was left reading the wrong value. */}
        <NewSessionSplit
          personas={personas}
          current={browseKey}
          onNew={props.onNewSession}
          onManage={props.onManagePersonas}
          onNewAssistant={() => setCreatingAssistant(true)}
          onNewProject={() => props.onNewProject(browseKey)}
        />

        {railRows(m, shownRail)}

        {/* More: a menu, not an inline disclosure. It holds whatever you have chosen not to
            keep on screen, plus the way to change that choice — and it is always present,
            even with nothing hidden, because "Edit sidebar" has to live somewhere you can
            find without first hiding something.

            Expanding inline pushed the session list down by however many rows were behind
            it, so opening More moved the thing you were about to click. A popover does not
            move anything. */}
        <div className="relative">
          <button
            ref={moreButtonRef}
            className={
              "nav-row w-full flex items-center gap-2.5 px-2 rounded-row text-md text-left hover:bg-solid hover:text-ink " +
              (moreOpen || hiddenActive ? "text-ink" : "text-muted") +
              (hiddenActive && !moreOpen ? " bg-line" : "")
            }
            data-testid="nav-more"
            aria-haspopup="menu"
            aria-expanded={moreOpen}
            aria-controls={moreOpen ? "rail-more" : undefined}
            onClick={() => setMoreOpen((v) => !v)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown" && !moreOpen) {
                e.preventDefault();
                setMoreOpen(true);
              }
            }}
          >
            {/* Dots, not a chevron: this opens a menu beside the rail. A chevron that turned over
                promised the rows would unfold in place, and they never did. */}
            <Icon name="moreHorizontal" size={ICON.md} className="shrink-0" />
            <span className="flex-1">More</span>
            {/* Any count on a hidden row is promoted here, so choosing to hide a surface can
                never hide the fact that something is waiting on it. */}
            {!moreOpen && <AttnBadge n={hiddenAttention} />}
          </button>

          {moreOpen && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setMoreOpen(false)} />
              {/* Opens to the right of the rail. Dropped below the row, it covered Pinned,
                  Scheduled and the top of Recent — the lists you would reach for next. */}
              <div
                id="rail-more"
                role="menu"
                aria-label="More"
                data-testid="more-menu"
                ref={moreMenuRef}
                onKeyDown={(e) => menuKeys(e, () => setMoreOpen(false), moreButtonRef.current)}
                className="rail-flyout absolute top-0 z-50 w-56 rounded-xl border border-line bg-panel shadow-xl p-1"
              >
                {hiddenRail.map((entry) => {
                  const { go, active } = navTarget(entry.id);
                  return (
                    <button
                      key={entry.id}
                      role="menuitem"
                      className={
                        "nav-row w-full flex items-center gap-2.5 px-2 rounded-row text-md text-left hover:bg-solid " +
                        (active ? "text-ink bg-line" : "text-ink")
                      }
                      data-testid={entry.testid}
                      title={entry.hint}
                      onClick={() => {
                        setMoreOpen(false);
                        go();
                      }}
                    >
                      <Icon name={entry.icon} size={ICON.md} className="shrink-0" />
                      <span className="flex-1">{entry.label}</span>
                      {entry.id === "inbox" ? <AttnBadge n={totalAttention} /> : null}
                    </button>
                  );
                })}
                {hiddenRail.length > 0 && <div className="my-1 border-t border-line" />}
                <button
                  role="menuitem"
                  data-testid="more-edit-sidebar"
                  className="nav-row w-full flex items-center gap-2.5 px-2 rounded-row text-md text-left text-ink hover:bg-solid"
                  onClick={() => {
                    setMoreOpen(false);
                    setEditSidebarOpen(true);
                  }}
                >
                  <Icon name="pencil" size={ICON.md} className="shrink-0" />
                  <span className="flex-1">Edit sidebar</span>
                </button>
              </div>
            </>
          )}
        </div>
      </nav>
  );
}
