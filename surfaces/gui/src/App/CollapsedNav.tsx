import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** While the nav is collapsed: a thin strip on the left edge that brings it back as a floating
 *  overlay on hover, and, on every surface but the session view, a button that does the same.
 *  The session view's topbar has its own [sidebar][+][search] cluster (§22), so a button there
 *  would be a second one for the same job. ⌘B works everywhere. */
export function CollapsedNav({ m }: { m: AppModel }) {
  if (!m.navCollapsed) return null;
  const peek = () => m.setNavPeek(true);
  return (
    <>
      <div className="nav-hover-zone" onMouseEnter={peek} aria-hidden="true" />
      {!m.navPeek && m.surface !== "session" && (
        <button
          className="nav-reveal-btn"
          onClick={m.toggleNav}
          onMouseEnter={peek}
          title="Show sidebar (⌘B)"
          aria-label="Show sidebar"
        >
          <Icon name="sidebar" size={ICON.md} />
        </button>
      )}
    </>
  );
}
