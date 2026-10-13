import { Icon, ICON, type IconName } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** The session's ⋯ menu: rename, pin, archive, delete. */
export function SessionMenu({ m }: { m: AppModel }) {
  const { setSessionMenu, sessionMenu, setRenamingTitle, activeInfo, togglePinned, sessionId,
      toggleArchived, deleteConversation } = m;
  return (
    <div className="relative" onMouseDown={(e) => e.stopPropagation()}>
              <button
                className="topbar-icon-btn"
                onClick={() => setSessionMenu((v) => !v)}
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
                  <div
                    className="absolute right-0 top-8 z-50 w-52 rounded-xl2 border border-line bg-panel shadow-xl p-1"
                    role="menu"
                    data-testid="session-menu"
                  >
                    {[
                      { id: "rename", label: "Rename", icon: "pencil" as IconName,
                        on: () => setRenamingTitle(activeInfo?.named ? activeInfo.title ?? "" : "") },
                      { id: "pin", label: activeInfo?.pinned ? "Unpin" : "Pin", icon: "pin" as IconName,
                        on: () => togglePinned(sessionId, !activeInfo?.pinned) },
                      { id: "archive", label: "Archive", icon: "archive" as IconName,
                        on: () => toggleArchived(sessionId, true) },
                    ].map((a) => (
                      <button
                        key={a.id}
                        role="menuitem"
                        data-testid={`session-menu-${a.id}`}
                        // Disabled until the session exists on the server: renaming or
                        // archiving a chat you have not sent anything in yet acts on an id
                        // the store has never seen.
                        disabled={!activeInfo}
                        className="w-full flex items-center gap-2.5 px-2 py-1.5 rounded-row text-md text-left text-ink hover:bg-solid disabled:opacity-40 disabled:cursor-default"
                        onClick={() => {
                          setSessionMenu(false);
                          a.on();
                        }}
                      >
                        <Icon name={a.icon} size={ICON.md} className="shrink-0 text-muted" />
                        <span className="flex-1">{a.label}</span>
                      </button>
                    ))}
                    <div className="my-1 border-t border-line" />
                    <button
                      role="menuitem"
                      data-testid="session-menu-delete"
                      disabled={!activeInfo}
                      className="w-full flex items-center gap-2.5 px-2 py-1.5 rounded-row text-md text-left text-danger hover:bg-solid disabled:opacity-40 disabled:cursor-default"
                      onClick={() => {
                        setSessionMenu(false);
                        deleteConversation(sessionId);
                      }}
                    >
                      <Icon name="trash" size={ICON.md} className="shrink-0" />
                      <span className="flex-1">Delete</span>
                    </button>
                  </div>
                </>
              )}
            </div>
  );
}
