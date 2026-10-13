import { Icon, ICON } from "../Icon";
import type { SidebarModel } from "./model/useSidebarModel";

/** The brand bar: collapse, the wordmark, search, and the Chat/Code switch. */
export function SidebarBrand({ m }: { m: SidebarModel }) {
  const { hasBothKinds, surfaceTab, setSurfaceTab, props } = m;
  return (
    <div className="brand px-3.5 flex items-center gap-1" data-tauri-drag-region>
        {props.onCollapse && (
          <button
            className="nav-icon-btn nav-pin-btn"
            data-tip={props.collapsed ? "Dock sidebar (⌘B)" : "Collapse sidebar (⌘B)"}
            aria-label={props.collapsed ? "Dock sidebar" : "Collapse sidebar"}
            onClick={props.onCollapse}
          >
            <Icon name="sidebar" size={ICON.md} />
          </button>
        )}

        {/* Back to Home, forward to Workspace, in the order the tabs sit. A second way to
            the same two places, for the reason a browser keeps arrows by its tabs: your hand
            is already here. Each is live in exactly one of them, and the dead one says which
            side you are on. Added 2026-08-26, dropped when the tabs had labels, back now
            the labels are gone again (owner, 2026-10-13). */}
        {hasBothKinds && (
          <>
            <button
              className="nav-icon-btn"
              data-tip="Home"
              aria-label="Back to Home"
              data-testid="surface-back"
              disabled={surfaceTab === "chat"}
              onClick={() => setSurfaceTab("chat")}
            >
              <Icon name="arrowLeft" size={ICON.md} />
            </button>
            <button
              className="nav-icon-btn"
              data-tip="Workspace"
              aria-label="Forward to Workspace"
              data-testid="surface-forward"
              disabled={surfaceTab === "code"}
              onClick={() => setSurfaceTab("code")}
            >
              <Icon name="arrowRight" size={ICON.md} />
            </button>
          </>
        )}

        <span className="brand-wordmark">
          <span className="mark" aria-hidden="true">#</span>
          <span className="word">Superuser</span>
        </span>

        <span className="flex-1" />

        {/* Which half of the persona list the sidebar shows. An assistant talks; a
            workbench runs a shell and is bound to a folder — the list already drew
            that line, this makes it a choice instead of a scroll.

            Icons, with the name in a tooltip and in the accessible name (owner's call,
            2026-10-08). The labelled version spilled past the sidebar's edge in the
            desktop title bar, where 84px of the row belongs to the traffic lights.
            Hidden when there is only one kind, so it never offers a tab leading
            nowhere, and it follows the active session rather than fighting it. */}
        {hasBothKinds && (
          <span className="nav-seg" role="tablist" aria-label="Session kind">
            {(
              [
                { id: "chat", label: "Home", icon: "home" },
                { id: "code", label: "Workspace", icon: "code" },
              ] as const
            ).map((t) => (
              <button
                key={t.id}
                role="tab"
                aria-selected={surfaceTab === t.id}
                aria-label={t.label}
                data-tip={t.label}
                data-testid={`surface-tab-${t.id}`}
                className={"nav-seg-btn" + (surfaceTab === t.id ? " is-on" : "")}
                onClick={() => setSurfaceTab(t.id)}
              >
                <Icon name={t.icon} size={ICON.md} />
              </button>
            ))}
          </span>
        )}
      </div>
  );
}
