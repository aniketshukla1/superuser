import { useEffect, useState } from "react";
import { getConnectors, getSlackChannels } from "../../api";
import type { RosterHit } from "./RosterHit";

/** Live Slack channel-roster suggestions for what is typed into the picker. */
export function useSlackRoster(open: boolean, value: string) {
  // Slack workspaces for the roster lookup, learned lazily on first open (relay =
  // per-team; manual Socket Mode = the "default" flat workspace; [] = not connected).
  const [teams, setTeams] = useState<{ team_id: string; account: string }[] | null>(null);
  useEffect(() => {
    if (!open || teams !== null) return;
    getConnectors()
      .then((cs) => {
        const s = cs.find((c) => c.name === "slack");
        if (!s?.connected) return setTeams([]);
        setTeams(
          s.mode === "relay"
            ? (s.workspaces || []).map((w) => ({
                team_id: w.team_id,
                account: w.account || w.team_id,
              }))
            : [{ team_id: "default", account: s.account || "workspace" }],
        );
      })
      .catch(() => setTeams([]));
  }, [open, teams]);

  // Type a NAME → live roster suggestions (debounced; addresses/URLs skip the lookup).
  // `searching` keeps the wait VISIBLE: the first lookup per workspace is a cold
  // paginated conversations.list sweep — seconds on a big workspace — and a silent
  // gap reads as "the typeahead doesn't work" (owner report, 2026-07-09).
  const [roster, setRoster] = useState<RosterHit[]>([]);
  const [searching, setSearching] = useState(false);
  useEffect(() => {
    const name = value.trim().replace(/^#/, "");
    if (!open || !teams || teams.length === 0 || !name || name.includes(":") || name.includes("/")) {
      setRoster([]);
      setSearching(false);
      return;
    }
    setSearching(true);
    const t = setTimeout(async () => {
      const rows = await Promise.all(
        teams.map(async (tm) => {
          try {
            const r = await getSlackChannels(tm.team_id, name);
            return (r.ok ? r.channels || [] : []).map((c) => ({
              address:
                tm.team_id === "default" ? `slack:${c.id}` : `slack:${tm.team_id}/${c.id}`,
              name: c.name,
              workspace: teams.length > 1 ? tm.account : "",
              is_private: c.is_private,
              is_member: c.is_member,
            }));
          } catch {
            return [] as RosterHit[];
          }
        }),
      );
      setRoster(rows.flat().slice(0, 12));
      setSearching(false);
    }, 250);
    return () => clearTimeout(t);
  }, [value, open, teams]);
  return { roster, searching };
}
