import { useEffect, useState } from "react";
import { getRecentChannels, getSubscriptions, subscribeChannel, unsubscribeChannel, type RecentChannel, type Subscription } from "../../api";
import { platformOf } from "./ScopePicker";

/** The channels this session listens on, per connector, and adding/removing one. */
export function useChannelSubscriptions(sessionId: string, active: boolean, channelsFor: string | null) {
  const [subs, setSubs] = useState<Subscription[]>([]);
  const [recent, setRecent] = useState<RecentChannel[]>([]);
  const [draft, setDraft] = useState("");
  const [addErr, setAddErr] = useState<string | null>(null);
  const loadSubs = () => getSubscriptions().then(setSubs).catch(() => setSubs([]));
  useEffect(() => {
    if (!active) return;
    loadSubs();
    getRecentChannels().then(setRecent).catch(() => setRecent([]));
  }, [active]);
  const channelsOf = (connector: string) =>
    subs.filter((s) => s.session_id === sessionId && platformOf(s.channel) === connector);
  const addChannel = async () => {
    const raw = draft.trim();
    if (!raw || !channelsFor) return;
    const channel = raw.includes(":") || raw.startsWith("#") ? raw : `${channelsFor}:${raw}`;
    const r = await subscribeChannel(sessionId, channel);
    if (!r.ok) {
      setAddErr(r.error || "Couldn't add that channel.");
      return;
    }
    setAddErr(null);
    setDraft("");
    loadSubs();
  };
  const removeChannel = async (channel: string) => {
    await unsubscribeChannel(sessionId, channel);
    loadSubs();
  };

  return { recent, draft, setDraft, addErr, setAddErr, channelsOf, addChannel, removeChannel };
}
