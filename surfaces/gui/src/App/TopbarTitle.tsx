import type { PointerEvent } from "react";
import { Icon, ICON, type IconName } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** Stops a press from also starting a window drag. */
const noDrag = (e: PointerEvent) => e.stopPropagation();

/** [sidebar] [+ new session] [search], shown while the sidebar is collapsed (§22). */
function CollapsedCluster({ m }: { m: AppModel }) {
  const buttons: { icon: IconName; label: string; tip: string; run: () => void }[] = [
    { icon: "sidebar", label: "Show sidebar", tip: "Show sidebar (⌘B)", run: m.toggleNav },
    { icon: "plus", label: "New session", tip: "New session", run: () => m.startNewSession() },
    { icon: "search", label: "Search", tip: "Search", run: () => m.setSearchOpen(true) },
  ];
  return (
    <div className="flex gap-1 items-center" data-testid="topbar-cluster" onPointerDown={noDrag}>
      {buttons.map((b) => (
        <button key={b.icon} className="topbar-icon-btn" onClick={b.run} aria-label={b.label} data-tip={b.tip}>
          <Icon name={b.icon} size={ICON.md} />
        </button>
      ))}
    </div>
  );
}

/** The conversation's name: a text box while renaming, a button that starts renaming once the
 *  conversation exists, and plain text before that. */
function TitleText({ m }: { m: AppModel }) {
  const { activeInfo, activeTitle, commitTitle, renamingTitle, setRenamingTitle } = m;
  if (renamingTitle !== null) {
    return (
      <input
        className="main-title-input"
        aria-label="Conversation name"
        data-testid="title-input"
        autoFocus
        value={renamingTitle}
        placeholder="Name this conversation"
        onPointerDown={noDrag}
        onChange={(e) => setRenamingTitle(e.target.value)}
        onBlur={commitTitle}
        onKeyDown={(e) => {
          if (e.key === "Enter") commitTitle();
          else if (e.key === "Escape") setRenamingTitle(null);
        }}
      />
    );
  }
  if (!activeInfo) {
    return (
      <span className="main-title-text title-ghost" data-tip={activeTitle}>
        {activeTitle}
      </span>
    );
  }
  // Renamed where it is read; pressing it names the conversation instead of dragging the window.
  return (
    <button
      className="is-editable main-title-text"
      data-testid="title-button"
      aria-label={`Rename “${activeTitle}”`}
      data-tip={`${activeTitle} — click to rename`}
      onPointerDown={noDrag}
      onClick={() => setRenamingTitle(activeInfo.named ? activeInfo.title ?? "" : "")}
    >
      {activeTitle}
    </button>
  );
}

/** The left of the top bar: the nav controls and the editable conversation title. There is no
 *  session-settings row here any more (§32): "what can this touch" is shown on the panel's
 *  Access header, and the panel toggle is the way in. */
export function TopbarTitle({ m }: { m: AppModel }) {
  const facts = m.subtitleShown;
  return (
    <div className="main-topbar-side" onPointerDown={m.beginWindowDrag}>
      {m.navCollapsed && <CollapsedCluster m={m} />}
      {/* The title and its facts sit LEFT-aligned on one line, as design/Main.dc.html draws them:
          inside the left region, because as a sibling between two flex:1 regions it was centred
          whatever its own alignment said. The facts are a chip, since plain grey text after a
          bold title on the same line reads as part of the title. */}
      <div className="main-title" onPointerDown={m.beginWindowDrag}>
        <h1 className="main-title-h">
          <TitleText m={m} />
        </h1>
        {/* Facts only, nothing to click (the assistant page they opened is hidden in this
            release, owner 2026-07-22). The assistant's name is left out when the heading
            already is that name; it read "Main · Main". */}
        {m.hasHistory && facts.length > 0 && (
          <span className="title-sub" data-testid="session-subtitle">
            {facts.join(" · ")}
          </span>
        )}
      </div>
    </div>
  );
}
