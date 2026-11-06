import { Icon, ICON, type IconName } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

const ROW = "hover:bg-solid disabled:cursor-default flex gap-2.5 items-center disabled:opacity-40 px-2 py-1.5 rounded-row text-left text-md w-full";

type Action = { id: string; label: string; icon: IconName; run: () => void };

/** The session's ⋯ menu: rename, pin, archive and delete. */
export function SessionMenu({ m }: { m: AppModel }) {
  const { activeInfo, sessionId, sessionMenu, setSessionMenu } = m;
  const actions: Action[] = [
    { id: "rename", label: "Rename", icon: "pencil", run: () => m.setRenamingTitle(activeInfo?.named ? activeInfo.title ?? "" : "") },
    { id: "pin", label: activeInfo?.pinned ? "Unpin" : "Pin", icon: "pin", run: () => m.togglePinned(sessionId, !activeInfo?.pinned) },
    { id: "archive", label: "Archive", icon: "archive", run: () => m.toggleArchived(sessionId, true) },
  ];
  /** Closes the menu, then does the thing. */
  const choose = (run: () => void) => () => {
    setSessionMenu(false);
    run();
  };
  // Every entry waits until the session exists on the server: renaming or archiving a chat
  // with nothing sent yet would act on an id the store has never seen.
  const unsaved = !activeInfo;
  return (
    <div className="relative" onMouseDown={(e) => e.stopPropagation()}>
      <button
        className="topbar-icon-btn"
        onClick={() => setSessionMenu((open) => !open)}
        aria-label="Session actions"
        aria-haspopup="menu"
        aria-expanded={sessionMenu}
        data-tip="Session actions"
        data-testid="topbar-session-menu"
      >
        <Icon name="dots" size={ICON.md} />
      </button>
      {sessionMenu && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setSessionMenu(false)} />
          <div className="absolute bg-panel border border-line p-1 right-0 rounded-xl2 shadow-xl top-8 w-52 z-50" role="menu" data-testid="session-menu">
            {actions.map((action) => (
              <button
                key={action.id}
                role="menuitem"
                data-testid={`session-menu-${action.id}`}
                disabled={unsaved}
                className={`${ROW} text-ink`}
                onClick={choose(action.run)}
              >
                <Icon name={action.icon} size={ICON.md} className="shrink-0 text-muted" />
                <span className="flex-1">{action.label}</span>
              </button>
            ))}
            <div className="border-line border-t my-1" />
            <button role="menuitem" data-testid="session-menu-delete" disabled={unsaved} className={`${ROW} text-danger`} onClick={choose(() => m.deleteConversation(sessionId))}>
              <Icon name="trash" size={ICON.md} className="shrink-0" />
              <span className="flex-1">Delete</span>
            </button>
          </div>
        </>
      )}
    </div>
  );
}
