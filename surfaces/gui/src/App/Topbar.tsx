import type { AppModel } from "./model/useAppModel";
import { TopbarTitle } from "./TopbarTitle";
import { TopbarActions } from "./TopbarActions";

/** The chat's top bar: title on the left, tools and the session menu on the right. */
export function Topbar({ m }: { m: AppModel }) {
  return (
    <div className="main-topbar">
          {/* Left: the contextual cluster — [sidebar] [+ new session] [search] — rendered ONLY
              while the sidebar is collapsed (§22; the expanded sidebar already owns those
              actions). Clicks must not start a window drag. */}
          <TopbarTitle m={m} />
          {/* Right: session-settings icon (§23) + panel toggle. Model/mode/persona chrome is
              gone — the facts live in the subtitle, the controls in the composer (§22). */}
          <TopbarActions m={m} />
        </div>
  );
}
