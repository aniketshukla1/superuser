import type { Persona, RecentWorkspace, SurfaceVisibility } from "../../api";
import type { SessionInfo } from "../../types";
import { shortPersonaName } from "../../personaScope";
import { ConnectorIcon } from "../../connectors/ConnectorIcon";
import type { IconName } from "../Icon";
import { personaGlyph } from "../personaIcon";

// Session surfaces shown as accordions, in display order. The surfaced personas drive this list
// (so third-party / Ops personas appear); the hardcoded set is the fallback before personas load.
export const SURFACES: { key: string; label: string; icon: IconName; cls: string; kind?: string }[] = [
  { key: "cowork", label: "Assistant", icon: "diamond", cls: "ico-cowork" },
  { key: "chat", label: "Chat", icon: "chat", cls: "ico-chat" },
  { key: "code", label: "Code", icon: "code", cls: "ico-code" },
];

export const surfaceFromPersona = (p: Persona) => ({
  kind: p.kind || "assistant",
  key: p.id,
  label: shortPersonaName(p.name, p.id),
  icon: personaGlyph(p.icon, p.family),
  cls: `ico-${p.icon || "cowork"}`,
});

// Attention = Inbox items awaiting a session (an accent count that bubbles session → persona →
// footer Inbox — all views of the one Inbox queue, never a second list).
export function AttnBadge({ n }: { n: number }) {
  if (!n) return null;
  return (
    <span
      className="text-xs font-semibold text-ink bg-faint/30 rounded-full px-1.5 leading-[15px] shrink-0"
      title={`${n} awaiting your attention`}
    >
      {n > 99 ? "99+" : n}
    </span>
  );
}

/** The keys a role="menu" list answers to: arrows and Home/End move between items, Escape
 *  closes and hands focus back to the control that opened it (it used to fall to the page
 *  body), and Tab closes so focus can carry on past the menu. */
export function menuKeys(e: React.KeyboardEvent<HTMLElement>, close: () => void, returnTo: HTMLElement | null) {
  const items = [...e.currentTarget.querySelectorAll<HTMLElement>("[role=menuitem]:not(:disabled)")];
  const at = items.indexOf(document.activeElement as HTMLElement);
  const move = (i: number) => {
    e.preventDefault();
    items[(i + items.length) % items.length]?.focus();
  };
  if (e.key === "ArrowDown") move(at + 1);
  else if (e.key === "ArrowUp") move(at < 0 ? items.length - 1 : at - 1);
  else if (e.key === "Home") move(0);
  else if (e.key === "End") move(items.length - 1);
  else if (e.key === "Escape") {
    e.preventDefault();
    close();
    returnTo?.focus();
  } else if (e.key === "Tab") close();
}

// UX-023: unseen-run count on a Scheduled entry. Deliberately QUIET — same neutral
// treatment as the attention badge; failure only colors the tooltip's words, not the
// sidebar (owner call 2026-07-20: no color, and the entry alone carries the count).
export function UnseenBadge({ n, failed }: { n: number; failed?: boolean }) {
  if (!n) return null;
  return (
    <span
      className="text-xs font-semibold text-ink bg-faint/30 rounded-full px-1.5 leading-[15px] shrink-0"
      title={failed ? `${n} new run${n > 1 ? "s" : ""} — the latest failed` : `${n} new run${n > 1 ? "s" : ""}`}
    >
      {n > 99 ? "99+" : n}
    </span>
  );
}

// Liveness = working (in-flight turn) / sleeping (a self-wake is pending). A count-less dot that
// never bubbles — it says "this is alive", not "this needs you".
/**
 * Up to two initials for the account chip.
 *
 * One letter is ambiguous the moment a person has more than one account, and the
 * chip is the only thing identifying which is signed in. Falls back to one letter
 * for a single-word name, and to "?" for nothing at all — never to an empty circle,
 * which reads as a broken avatar rather than an unknown one.
 */
export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  const letters = parts.length === 1 ? parts[0].slice(0, 1) : parts[0][0] + parts[parts.length - 1][0];
  return letters.toUpperCase();
}

export function LiveDot({ state }: { state?: "working" | "sleeping" | "idle" }) {
  if (state !== "working" && state !== "sleeping") return null;
  return state === "working" ? (
    <span className="w-1.5 h-1.5 rounded-full bg-accent animate-pulse shrink-0" title="Working now" />
  ) : (
    <span
      className="w-1.5 h-1.5 rounded-full bg-faint/60 shrink-0"
      title="Sleeping (will wake itself)"
    />
  );
}

// §31: a session spawned by a platform mention wears its platform's logo, right-aligned beside
// the title cluster (owner call 2026-07-13). Slack today; the origin key is the platform id.
export function OriginIcon({ s }: { s: SessionInfo }) {
  if (s.origin !== "slack") return null;
  return (
    <ConnectorIcon
      connector={{ logo: "slack", brand_color: "#611f69" }}
      size={12}
      title={s.origin_label || "From Slack"}
    />
  );
}

// A subscribed-connector presence dot (right edge of a row). Brand-colorless here — the sidebar
// isn't passed the connector registry — so it reads as a neutral "listening on a channel" dot.
export function ConnectorDot({ subs }: { subs?: string[] }) {
  if (!subs || subs.length === 0) return null;
  return (
    <span
      className="w-1.5 h-1.5 rounded-full bg-faint shrink-0"
      data-brand={subs[0]}
      title={subs.join(", ")}
    />
  );
}

export interface Props {
  agent: string;
  workspace: string;
  surfaces: SurfaceVisibility;
  sessions: SessionInfo[];
  projects: RecentWorkspace[];
  activeSession: string;
  onSwitchAgent: (agent: string) => void;
  onNewSession: (agent: string) => void;
  onSelectSession: (id: string, workspace: string, agent: string) => void;
  onNewProject: (persona: string) => void;
  /* Start a session already bound to a folder — the per-project `+`. Distinct from
     onNewProject, which opens the folder gate because no folder is known yet. */
  onNewInProject: (workspace: string, persona: string) => void;
  onRenameSession: (id: string, title: string) => void;
  onDeleteSession: (id: string) => void;
  onArchiveSession: (id: string, archived: boolean) => void;
  onTogglePin: (id: string, pinned: boolean) => void;
  onManage: () => void;
  // Grouped-nav gear + New-session menu's "Manage personas…" entry points (§7).
  onOpenPersona: (id: string) => void;
  onManagePersonas: () => void;
  onOpenScheduled: () => void;
  // Scheduled-band "+": open the Automations surface with the new-automation form up.
  onNewAutomation: () => void;
  // Scheduled-band row click: open the Automations surface ON that automation (UX-023).
  onOpenAutomation: (id: string) => void;
  onOpenIntegrations: () => void;
  onOpenAudit: () => void;
  onOpenInbox: () => void;
  onOpenBrain: () => void;
  scheduledActive: boolean;
  integrationsActive: boolean;
  auditActive: boolean;
  inboxActive: boolean;
  brainActive: boolean;
  // Collapse controls (⌘B / hover-peek). `onCollapse` docks/undocks; `onPeekLeave` hides the
  // floating peek when the pointer leaves the panel.
  collapsed?: boolean;
  onCollapse?: () => void;
  onPeekLeave?: () => void;
}

// Compact age for project session rows: "now" / "5m" / "6h" / "3d" / "2w" / "4mo" / "2y".
export const compactAge = (iso?: string | null): string => {
  if (!iso) return "";
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "";
  const secs = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (secs < 60) return "now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h`;
  const days = Math.floor(hrs / 24);
  if (days < 7) return `${days}d`;
  const weeks = Math.floor(days / 7);
  if (days < 30) return `${weeks}w`;
  const months = Math.floor(days / 30);
  if (days < 365) return `${months}mo`;
  return `${Math.floor(days / 365)}y`;
};
