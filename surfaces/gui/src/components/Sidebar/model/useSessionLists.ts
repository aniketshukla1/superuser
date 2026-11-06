import { RAIL_NAV } from "../../../nav";
import { useEffect } from "react";
import { isProjectScoped } from "../../../personaScope";
import type { SessionInfo } from "../../../types";
import type { Props } from "../Props";
import type { useAccountMenu } from "./useAccountMenu";
import type { useAutomationsList } from "./useAutomationsList";
import type { useRowMenu } from "./useRowMenu";
import type { usePersonaLayout } from "./usePersonaLayout";
import type { useProjects } from "./useProjects";

/** The account summary, attention counts, and the session lists the sidebar draws. */
export function useSessionLists(prev: { props: Props } & ReturnType<typeof useAccountMenu> & ReturnType<typeof useAutomationsList> & ReturnType<typeof useRowMenu> & ReturnType<typeof usePersonaLayout> & ReturnType<typeof useProjects>) {
  const { cloud, navVisible, navTarget, inboxUnlocked, setInboxUnlocked, browseKey, personaOf,
    personaVisible, props } = prev;
  // Display identity for the account row: the cloud profile only carries the email, so the
  // row shows the capitalized local part ("rohit@…" → "Rohit"); the menu header shows it all.
  const accountEmail = cloud?.signed_in ? cloud.account : "";
  const accountName = accountEmail
    ? accountEmail.split("@")[0].replace(/^./, (c) => c.toUpperCase())
    : "";

  // Roll the per-session attention/liveness up to the persona header and the footer Inbox: the
  // accent count bubbles (sum), the liveness dot aggregates (working wins over sleeping).
  const attnByPersona = new Map<string, number>();
  const liveByPersona = new Map<string, "working" | "sleeping">();
  let totalAttention = 0;
  for (const s of props.sessions) {
    if (s.session_id.startsWith("__") || s.archived) continue;
    const a = s.attention || 0;
    if (a > 0) {
      attnByPersona.set(s.agent, (attnByPersona.get(s.agent) || 0) + a);
      totalAttention += a;
    }
    if (s.liveness === "working") liveByPersona.set(s.agent, "working");
    else if (s.liveness === "sleeping" && liveByPersona.get(s.agent) !== "working")
      liveByPersona.set(s.agent, "sleeping");
  }

  // Counts belonging to rows the More disclosure is currently hiding, summed onto
  // the More row itself. Derived from the tier list rather than hardcoded, so
  // moving an entry between tiers cannot strand its badge somewhere invisible.
  // Inbox is the only surface with a count today, and it is primary — so this is
  // 0 in practice and exists to stop the next badge being swallowed silently.
  // Which rows are on screen is the user's choice now (navPrefs), so "hidden" is computed
  // from that rather than from nav.ts's tier — tier is only what a fresh install starts with.
  const shownRail = RAIL_NAV.filter((e) => navVisible.includes(e.id));
  const hiddenRail = RAIL_NAV.filter((e) => !navVisible.includes(e.id));
  const hiddenAttention = hiddenRail.reduce(
    (n, e) => n + (e.id === "inbox" ? totalAttention : 0),
    0,
  );

  // Open More when the surface you are on lives inside it, so the current location
  // is never hidden behind a collapsed row — landing on Privacy from a link would
  // otherwise highlight nothing at all.
  // Landing on a surface you have hidden used to force the More disclosure open so the
  // current location was never invisible. More is a menu now, and a menu that opens itself
  // is a popover appearing over your work unasked — so instead the More row itself lights up
  // as active, which says the same thing without seizing the screen.
  const hiddenActive = hiddenRail.some((e) => navTarget(e.id).active);

  // First pending item ever observed → the inbox chip unlocks and stays (§26 sticky unlock).
  useEffect(() => {
    if (totalAttention > 0 && !inboxUnlocked) {
      localStorage.setItem("superuser:inbox-unlocked", "1");
      setInboxUnlocked(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [totalAttention]);

  // Body data is keyed to the BROWSED persona (only one body renders at a time). Pinned sessions are
  // EXCLUDED here: they live in the cross-persona Pinned band only, so they don't repeat inside the
  // persona group / project list (matching the flat layout's Recent, which also drops pinned).
  // Sub-threads are excluded here too — they render nested under their parent row
  // (threadRows), so a fan-out reads the same in the grouped accordion as it does
  // in the flat list.
  const all = props.sessions.filter(
    (s) => s.agent === browseKey && !s.session_id.startsWith("__") && !s.parent_id,
  );
  const mine = all.filter((s) => !s.archived && !s.pinned);
  const archived = all.filter((s) => s.archived);
  // Only PROJECT-SCOPED personas group sessions by project (git-bound Code, project-bound Ops).
  // Scratch/deliverable conversations are orphan (each has its own per-conversation scratch dir),
  // so they list flat. Workspace-aware (not id-aware) — any git/project persona gets Projects.
  const workspaceSurface = isProjectScoped(personaOf(browseKey));

  // Search now lives in the SearchModal (command-palette overlay), so the sidebar lists never filter
  // in place — these stay constant and the `.filter(matches)` / `normalizedQuery ? …` call sites
  // below are intentional no-ops kept to avoid churn.
  const normalizedQuery = "";
  const matches = (_s: SessionInfo) => true;

  // Recent = every non-pinned, non-archived, real session across ALL personas, newest first
  // (by updated_at; missing timestamps keep store order), search-filtered. Drives the flat layout.
  const recentSessions = [...props.sessions]
    .filter((s) => !s.archived && !s.session_id.startsWith("__") && !s.pinned)
    // Sub-threads aren't top-level rows — they hang under the session they were
    // split out of (see threadRows).
    .filter((s) => !s.parent_id)
    .filter((s) => personaVisible(s.agent))
    .filter(matches)
    .sort((a, b) => (b.updated_at || "").localeCompare(a.updated_at || ""));

  // parent session id → its sub-threads, oldest first so they read in the order
  // the tasks were written.
  const threadsByParent = new Map<string, SessionInfo[]>();
  for (const s of props.sessions) {
    if (!s.parent_id || s.archived) continue;
    const kids = threadsByParent.get(s.parent_id) ?? [];
    kids.push(s);
    threadsByParent.set(s.parent_id, kids);
  }
  for (const kids of threadsByParent.values()) {
    kids.sort((a, b) => (a.updated_at || "").localeCompare(b.updated_at || ""));
  }

  return { accountEmail, accountName, attnByPersona, liveByPersona, totalAttention, shownRail, hiddenRail, hiddenAttention, hiddenActive, mine, archived, workspaceSurface, normalizedQuery, matches, recentSessions, threadsByParent };
}
