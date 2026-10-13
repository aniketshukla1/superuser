import type { InboxItem, Persona } from "../../api";
import { Icon, ICON } from "../Icon";
import { requesterLabel } from "../../inboxText";
import { ICON_FOR } from "./ICON_FOR";

/** The originating-session chip: persona icon + session title, clickable to open that session. */
export function SessionChip({
  it,
  personas,
  onOpenSession,
}: {
  it: InboxItem;
  personas: Persona[] | null;
  onOpenSession: (sessionId: string, workspace: string, agent: string) => void;
}) {
  const exists = it.session_exists !== false;
  const p = personas?.find((x) => x.id === it.session_agent);
  // A name, never an id: automation runs were listed as `__run__run-ed914dd092`.
  const label = requesterLabel(it);
  const icon = (p && ICON_FOR[p.icon]) || "diamond";
  const cls = `ico-${p?.icon || "cowork"}`;
  return (
    <button
      className="inbox-session-chip"
      title={exists ? `Open “${label}”` : "Session unavailable"}
      disabled={!exists}
      onClick={() =>
        exists && onOpenSession(it.session_id, it.session_workspace || "", it.session_agent || "cowork")
      }
    >
      <span className={"inbox-chip-ico " + cls}>
        <Icon name={icon} size={ICON.sm} />
      </span>
      <span className="inbox-chip-label">{label}</span>
      {exists && <Icon name="chevronRight" size={ICON.md} className="inbox-chip-go" />}
    </button>
  );
}
