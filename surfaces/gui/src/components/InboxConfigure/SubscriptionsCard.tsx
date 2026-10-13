import { useEffect, useState } from "react";
import { getDmRoute, getRecentChannels, getSessions, getSubscriptions, setDmRoute, subscribeChannel, unsubscribeChannel, type RecentChannel, type Subscription } from "../../api";
import type { SessionInfo } from "../../types";
import { ChannelPicker } from "../SubscriptionsChip";
import { Icon, ICON } from "../Icon";
import { CARD } from "../../ui";
import { SELECT } from "./InboxRoutingCard";

// Which session handles incoming DMs to the bot. None → DMs park in the Unrouted section below.
export function DmRouteCard() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [dm, setDm] = useState<string>("");

  const load = () => {
    getSessions().then(setSessions).catch(() => setSessions([]));
    getDmRoute().then((s) => setDm(s || "")).catch(() => setDm(""));
  };
  useEffect(() => {
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, []);

  const real = sessions.filter((s) => !s.session_id.startsWith("__"));
  const choose = async (sessionId: string) => {
    setDm(sessionId);
    await setDmRoute(sessionId);
    load();
  };

  return (
    <div className={CARD + " p-4"}>
      <div className="font-semibold text-md mb-1">Direct messages</div>
      <p className="text-sm text-muted mb-3">
        The conversation that answers direct messages to the bot. With none, they wait under
        Unrouted below.
      </p>
      <div className="flex items-center gap-2">
        <span className="text-muted shrink-0">
          <Icon name="chat" size={ICON.md} />
        </span>
        <select
          className={"flex-1 min-w-0 " + SELECT}
          aria-label="Conversation that answers direct messages"
          value={dm}
          onChange={(e) => choose(e.target.value)}
        >
          <option value="">None — keep them under Unrouted</option>
          {real.map((s) => (
            <option key={s.session_id} value={s.session_id}>
              {s.title || s.session_id}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

// Which sessions listen to which channels (inbound), and where each routes its Inbox (outbound).
// Subscriptions can be created by the agent (it asks you via ask_user) or added here directly.
export function SubscriptionsCard() {
  const [subs, setSubs] = useState<Subscription[] | null>(null);
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [recent, setRecent] = useState<RecentChannel[]>([]);
  const [addSession, setAddSession] = useState("");
  const [addChannel, setAddChannel] = useState("");

  const load = () => {
    getSubscriptions().then(setSubs).catch(() => setSubs([]));
    getSessions().then(setSessions).catch(() => setSessions([]));
    getRecentChannels().then(setRecent).catch(() => setRecent([]));
  };
  useEffect(() => {
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, []);

  const real = sessions.filter((s) => !s.session_id.startsWith("__"));
  const add = async () => {
    if (!addSession || !addChannel.trim()) return;
    await subscribeChannel(addSession, addChannel.trim());
    setAddChannel("");
    load();
  };
  const remove = async (sessionId: string, channel: string) => {
    await unsubscribeChannel(sessionId, channel);
    load();
  };

  return (
    <div className={CARD + " mb-4 overflow-hidden"}>
      <div className="px-4 py-3 border-b border-line flex items-center gap-2">
        <span className="text-muted shrink-0">
          <Icon name="plug" size={ICON.md} />
        </span>
        <span className="font-semibold text-md">Channel subscriptions</span>
        <span className="text-sm text-muted">Conversations that read a channel and can answer in it</span>
      </div>

      {subs && subs.length > 0 ? (
        <table className="w-full text-md">
          <thead className="text-xs text-faint">
            <tr className="text-left">
              <th className="font-medium px-4 py-2">Conversation</th>
              <th className="font-medium px-4 py-2">Listens to</th>
              <th className="font-medium px-4 py-2">Approvals go to</th>
              <th className="px-4 py-2">
                <span className="sr-only">Remove</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {subs.map((s, i) => (
              <tr className="border-t border-line" key={i}>
                <td className="px-4 py-2.5 truncate max-w-[12rem]" title={s.session_title}>
                  {s.session_title}
                </td>
                <td className="px-4 py-2.5">
                  <span className="inline-flex items-center gap-1.5" title={s.channel}>
                    <span className="text-muted shrink-0">
                      <Icon name="plug" size={ICON.md} />
                    </span>
                    {s.channel_name ? `#${s.channel_name}` : s.channel}
                    {s.channel_name && (
                      <span className="text-xs text-faint">{s.channel}</span>
                    )}
                  </span>
                  {s.collision && (
                    <span
                      className="ml-1.5 text-xs text-warnInk bg-warnSoft/70 border border-warnInk/15 rounded px-1.5 py-0.5"
                      title="This channel is also your Inbox-routing target — inbound and outbound on one channel conflate broadcast with request/reply."
                    >
                      ⚠ collides
                    </span>
                  )}
                </td>
                <td className="px-4 py-2.5 text-muted">{s.routing_target || "—"}</td>
                <td className="px-4 py-2.5 text-right">
                  <button
                    className="nav-icon-btn"
                    title="Unsubscribe"
                    aria-label={`Unsubscribe ${s.session_title || "this conversation"} from ${s.channel_name ? `#${s.channel_name}` : s.channel}`}
                    onClick={() => remove(s.session_id, s.channel)}
                  >
                    <Icon name="x" size={ICON.md} />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <div className="px-4 py-3 text-sm text-muted">
          No channel subscriptions yet — add one below or ask an assistant to watch a channel.
        </div>
      )}

      <div className="border-t border-line px-4 py-3 flex items-center gap-2 flex-wrap">
        <select
          className={SELECT}
          aria-label="Conversation"
          value={addSession}
          onChange={(e) => setAddSession(e.target.value)}
        >
          <option value="">Choose a conversation…</option>
          {real.map((s) => (
            <option key={s.session_id} value={s.session_id}>
              {s.title || s.session_id}
            </option>
          ))}
        </select>
        <ChannelPicker value={addChannel} onChange={setAddChannel} recent={recent} onSubmit={add} />
        <button className="btn-primary sm" disabled={!addSession || !addChannel.trim()} onClick={add}>
          Subscribe
        </button>
      </div>
    </div>
  );
}
