import type { AppModel } from "./model/useAppModel";
import { TopbarActions } from "./TopbarActions";
import { TopbarTitle } from "./TopbarTitle";

/** The chat's top bar. On the left, the heading, plus [sidebar] [+ new session] [search] while
 *  the sidebar is collapsed (§22; an open sidebar has those already), none of which may start
 *  a window drag. On the right, the panel's tools and toggle; the model, mode and assistant
 *  controls went to the composer, and the facts to the subtitle (§22). */
export function Topbar({ m }: { m: AppModel }) {
  return (
    <div className="main-topbar">
      <TopbarTitle m={m} />
      <TopbarActions m={m} />
    </div>
  );
}
