import { useEffect } from "react";
import { getConnectors, type CloudStatus, type Connector, type RecentChannel, type Subscription } from "../../api";
import { Icon, ICON } from "../Icon";
import { ConnectSetup } from "../ManageTabs";
import { ChannelPicker } from "../SubscriptionsChip";
import { SEC_H } from "../../ui";
import { BTN_ACCENT } from "./ScopePicker";

// Connect-in-context (§32 child view): the same ConnectSetup the global Connectors page uses,
// hosted inline in the section so connecting never navigates away. Managed connects complete
// out-of-band (browser → broker → sidecar), so poll until the connector flips.
export function ConnectInline({
  c,
  cloud,
  onDone,
  onBack,
}: {
  c: Connector;
  cloud: CloudStatus | null;
  onDone: () => void;
  onBack: () => void;
}) {
  useEffect(() => {
    const t = setInterval(async () => {
      try {
        const list = await getConnectors();
        if (list.find((x) => x.name === c.name)?.connected) onDone();
      } catch {
        /* poll again */
      }
    }, 2500);
    return () => clearInterval(t);
  }, [c.name, onDone]);

  return (
    <div>
      <button
        className="inline-flex items-center gap-1 text-sm text-faint hover:text-ink mb-2"
        onClick={onBack}
        aria-label="Back to sources"
      >
        <Icon name="arrowLeft" size={ICON.md} /> Connect {c.title}
      </button>
      {c.blurb && <p className="text-sm text-muted mb-1 leading-prose">{c.blurb}</p>}
      <div className="-mx-2">
        <ConnectSetup c={c} cloud={cloud} onConnected={onDone} />
      </div>
      {/* Scope semantics, stated once (owner ask 2026-07-13): connecting is account-level,
          the toggle above is what scopes it to a session. */}
      <p className="text-xs text-faint mt-2 leading-snug">
        Connecting makes {c.title} available to all your assistants — the toggle in this list
        controls just this session.
      </p>
    </div>
  );
}

// The per-connector channels drill-down (§32 child view): which channels THIS session listens
// to on a two-way messaging connector (Slack/Telegram).
export function ChannelsInline({
  label,
  channels,
  recent,
  draft,
  onDraft,
  onAdd,
  error,
  onRemove,
  onBack,
}: {
  label: string;
  channels: Subscription[];
  recent: RecentChannel[];
  draft: string;
  onDraft: (v: string) => void;
  onAdd: () => void;
  error?: string | null;
  onRemove: (channel: string) => void;
  onBack: () => void;
}) {
  return (
    <div>
      <button
        className="inline-flex items-center gap-1 text-sm text-faint hover:text-ink mb-2"
        onClick={onBack}
        aria-label="Back to sources"
      >
        <Icon name="arrowLeft" size={ICON.md} /> {label} channels
      </button>
      <div className={`${SEC_H} mb-1.5`}>Subscribed channels · {channels.length}</div>
      {channels.length === 0 ? (
        <div className="text-sm text-faint py-0.5">
          Not listening to any {label} channel yet.
        </div>
      ) : (
        <div className="space-y-1">
          {channels.map((s) => (
            <div className="flex items-center gap-1.5 py-1" key={s.channel}>
              <Icon name="plug" size={ICON.md} className="text-muted shrink-0" />
              <span className="min-w-0 flex-1 text-sm truncate" title={s.channel}>
                {s.channel_name ? `#${s.channel_name}` : s.channel}
              </span>
              {s.collision && (
                <span
                  className="text-xs text-warnInk bg-warnSoft/70 border border-warnInk/15 rounded px-1 shrink-0"
                  title="This channel is also this session's Inbox-routing target — inbound and outbound collide."
                >
                  ⚠
                </span>
              )}
              <button
                className="w-5 h-5 grid place-items-center text-faint hover:text-danger shrink-0"
                title="Stop listening"
                onClick={() => onRemove(s.channel)}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
      <div className={`${SEC_H} mt-3 mb-1.5`}>Add a channel</div>
      <div className="flex items-center gap-1.5">
        <ChannelPicker value={draft} onChange={onDraft} recent={recent} onSubmit={onAdd} />
        <button className={BTN_ACCENT} disabled={!draft.trim()} onClick={onAdd}>
          Add
        </button>
      </div>
      {error && (
        <p className="text-xs text-warnInk mt-1.5 leading-snug" data-testid="channel-add-error">
          {error}
        </p>
      )}
      <p className="text-xs text-faint mt-1.5 leading-snug">
        The agent receives messages posted to these channels. Removing one stops this session
        from listening — the connector stays connected.
      </p>
    </div>
  );
}
