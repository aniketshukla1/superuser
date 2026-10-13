import { useState } from "react";
import { addSlackApprovalOwner, resolveUnauthorized, removeSlackApprovalOwner, unsubscribeChannel, type ParkedMessage, type Subscription } from "../../../api";
import { PILL_ACCENT, PILL_LINE, ROW, XBTN } from "../ui";
import { LABEL, initials } from "./SlackDetail";
import { PersonPicker } from "./PersonPicker";

export function ApprovalOwnersRow({
  owners,
  names,
  installerId,
  installerName,
  editable,
  onChanged,
}: {
  owners: string[];
  names?: Record<string, string | null>;
  installerId?: string;
  installerName?: string;
  editable: boolean;
  onChanged: () => void;
}) {
  const [err, setErr] = useState<string | null>(null);
  const label = (u: string) =>
    names?.[u] || (u === installerId ? installerName || "You" : u);
  const remove = async (userId: string) => {
    const result = await removeSlackApprovalOwner(userId);
    if (!result.ok) {
      setErr(result.error || "could not remove approval owner");
      return;
    }
    setErr(null);
    onChanged();
  };
  return (
    <div className={ROW} data-testid="slack-approval-owners">
      <span className={LABEL}>Approvals</span>
      <span className="min-w-0 flex-1 flex flex-wrap items-center gap-1.5">
        {owners.length === 0 && (
          <span className="text-sm text-warnInk">
            Choose at least one owner before routing Inbox approvals to Slack.
          </span>
        )}
        {owners.map((u) => (
          <span
            key={u}
            className="inline-flex items-center gap-1.5 pl-1 pr-2 py-0.5 rounded-full bg-paper border border-line text-sm"
            title={`id ${u}`}
            data-testid={`approval-owner-${u}`}
          >
            <span className="w-5 h-5 rounded-full bg-accentSoft text-accent grid place-items-center text-xs font-bold">
              {initials(label(u))}
            </span>
            {label(u)}
            {u === installerId && <span className="text-xs text-faint">· installer</span>}
            {editable && (
              <button className={XBTN} title="remove approval owner" onClick={() => remove(u)}>
                ×
              </button>
            )}
          </span>
        ))}
        {editable && (
          <PersonPicker
            teamId={null}
            allowed={owners}
            onChanged={onChanged}
            onPick={(m) => addSlackApprovalOwner(m.id, m.name)}
            buttonLabel="＋ Add owner"
            testId="add-approval-owner"
          />
        )}
        {!editable && owners.length > 0 && (
          <span className="text-xs text-faint">Set by the workspace installer.</span>
        )}
        {err && <span className="basis-full text-xs text-warnInk">{err}</span>}
      </span>
    </div>
  );
}

export function WaitingRow({ m, onChanged }: { m: ParkedMessage; onChanged: () => void }) {
  const act = async (action: "dismiss" | "allow" | "allow_deliver") => {
    await resolveUnauthorized("slack", m.id, action);
    onChanged();
  };
  return (
    <div className={ROW + " bg-warnSoft/25"} data-testid={`waiting-${m.id}`}>
      <span className={LABEL}>Waiting</span>
      <span className="min-w-0 flex-1">
        <span className="font-medium text-md">{m.user_name || m.user_id}</span>{" "}
        <span className="text-sm text-muted">in {m.chat_name || m.chat_id}</span>
        <span className="block text-sm text-muted truncate">“{m.text}”</span>
      </span>
      <button
        className={PILL_ACCENT + " !py-1"}
        data-testid={`parked-allow-deliver-${m.id}`}
        title="Allow the sender and deliver this message now"
        onClick={() => act("allow_deliver")}
      >
        Allow & deliver
      </button>
      <button
        className={PILL_LINE + " !py-1"}
        data-testid={`parked-allow-${m.id}`}
        title="Allow the sender; this message is discarded"
        onClick={() => act("allow")}
      >
        Allow
      </button>
      <button className={XBTN + " px-1"} data-testid={`parked-dismiss-${m.id}`} title="Dismiss" onClick={() => act("dismiss")}>
        ×
      </button>
    </div>
  );
}

export function ListeningRows({ subs, onChanged }: { subs: Subscription[]; onChanged: () => void }) {
  if (subs.length === 0) return null;
  return (
    <div className={ROW} data-testid="listening-slack">
      <span className={LABEL}>Listening</span>
      <span className="min-w-0 flex-1 space-y-1">
        {subs.map((s) => (
          <span key={s.session_id + s.channel} className="flex items-center gap-2 text-sm">
            <span className="font-medium truncate" title={s.session_id}>
              {s.session_title || s.session_id}
            </span>
            <span className="text-faint">←</span>
            <span className="text-muted truncate" title={s.channel}>
              {s.channel_name ? `#${s.channel_name}` : s.channel}
            </span>
            <button
              className={XBTN + " ml-auto"}
              title="Unsubscribe this session"
              onClick={async () => {
                await unsubscribeChannel(s.session_id, s.channel);
                onChanged();
              }}
            >
              ×
            </button>
          </span>
        ))}
      </span>
    </div>
  );
}
