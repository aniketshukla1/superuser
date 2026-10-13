import type { ReactNode } from "react";
import type { Attachment, SessionUsage } from "../../types";
import type { ModeOption } from "../ComposerControls";

// The ways of working, and — below a divider, with its own verb — the guard you can
// switch off. "Full access" used to be the third item in a flat list of three, one
// arrow-key from the mode that asks permission for everything. It is the same server-side
// mode; only its place in the menu changed, because a list does not distinguish between
// "choose how it should work" and "turn the asking off".
//
// Plan was hidden in 2026-07 on the grounds that its approval flow was not polished
// enough to ship, and unhidden on the owner's ask (2026-08-26). The gap is real and the
// description states it rather than papering over it: there is no approve-this-plan
// button, so nothing runs until you change mode yourself. Everything else about the mode
// works — the server denies every non-read action in it, exactly as it does in Discuss,
// and adds "outline your approach as short steps first" to the prompt.
//
// Custom stays hidden: it is config.toml auto-allow rules with no in-app explanation, and
// a mode whose behaviour lives in a file the menu cannot show you is not a menu item. The
// server still honours it for a session already in it.
export const PERMISSION_OPTIONS: ModeOption[] = [
  { value: "discuss", label: "Discuss", description: "Chat and explore — no edits or commands" },
  { value: "plan", label: "Plan", description: "Outline the approach first; nothing runs until you switch mode" },
  { value: "interactive", label: "Ask for approval", description: "Ask before edits and commands" },
];
export const BYPASS_OPTION: ModeOption = {
  value: "auto",
  label: "Bypass permissions",
  description: "Run everything without asking",
};

// No hardcoded model fallback: until the server supplies the list (a few seconds after a
// cold app boot), the picker renders a disabled "Loading models…" chip. A baked-in list
// goes stale and silently offers ids the backend never confirmed (caught 2026-07-21).

// Drop the provider prefix for display (anthropic:claude-opus-4-8 → claude-opus-4-8); full id on hover.
export const shortModel = (m: string) => (m.includes(":") ? m.split(":").slice(1).join(":") : m);

// Identify an attachment by name + payload size so duplicates (e.g. the same file picked twice,
// or a prefill applied twice) collapse to one chip.
export const attKey = (a: Attachment) =>
  a.kind === "text"
    ? `t:${a.name}:${a.text?.length ?? 0}`
    : `${a.kind[0]}:${a.name}:${a.data_url?.length ?? 0}`;
export const mergeAttachments = (cur: Attachment[], add: Attachment[]): Attachment[] => {
  const seen = new Set(cur.map(attKey));
  return [...cur, ...add.filter((a) => !seen.has(attKey(a)))].slice(0, 8);
};

export interface Props {
  mode: string;
  model: string;
  models?: string[];
  modelLabels?: Record<string, string>; // curated display names (raw id when absent)
  // The model is FIXED once the session has history (§17): the picker renders ONLY on a fresh
  // session; after the first turn the fact lives in the topbar subtitle (§22) — no
  // interactive-then-disabled control.
  running: boolean;
  connected: boolean;
  // False when the default model's provider has no key — the composer shows a "connect a model"
  // banner and routes sends to setup (preserving the draft) instead of dropping them.
  modelReady?: boolean;
  onConnectModel?: () => void;
  onConfigureVoiceInput?: () => void;
  onSend: (text: string, attachments?: Attachment[], skill?: string) => void;
  /** Hands-free conversation: mic open, turns detected, replies spoken. */
  handsFree?: boolean;
  voiceState?: string;
  /** Why hands-free stopped or never started. Shown, never swallowed. */
  voiceError?: string | null;
  onToggleHandsFree?: () => void;
  // Feeds the "/" force-run popup (SKILLS-SPEC §4.1 #3): the popup lists this session's
  // effective skill menu. Absent (e.g. tests without sessions) → the popup never opens.
  sessionId?: string;
  onInterrupt: () => void;
  onModeChange: (mode: string) => void;
  onModelChange: (model: string) => void;
  // When set (Code/Cowork), the Mode menu is shown. The folder/roots + branch controls left the
  // composer for the Session settings drawer (§22) — folder access is standing session config.
  workspace?: string;
  /** The assistant this conversation is for. Feeds the "/" menu so a conversation
   *  with no turns yet still offers that assistant's procedures and not all of them. */
  agent?: string;
  // Unattended / send-approvals-to-Inbox — folded into the Mode menu (§22): "who approves, and
  // when" is one mental model. Absent handler = no toggle (e.g. Chat).
  unattended?: boolean;
  onUnattendedChange?: (on: boolean) => void;
  /** Opens the Connectors surface from the + menu. Absent = the row is not offered,
   *  rather than offered and inert. */
  onOpenConnectors?: () => void;
  approvalSlot?: ReactNode;
  // Push text + attachments into the composer (e.g. a start-panel task card). The `nonce` makes
  // repeated identical prefills re-apply; the user can still edit before sending.
  prefill?: { text: string; attachments?: Attachment[]; nonce: number };
  // Changes when the active conversation changes; clears any unsent draft.
  resetKey?: string;
  // Surface-specific hint shown in the empty textarea.
  placeholder?: string;
  // Per-session token usage (OPE-42) — absent/empty leaves the context ring unfilled and
  // its panel without the per-model breakdown (older servers, backends that don't report
  // usage, fresh sessions).
  usage?: SessionUsage;
  // Context-window size (tokens) of the ACTIVE model, from the curated matrix;
  // undefined hides the fill meter (unverified/custom models) but keeps the counts.
  contextWindow?: number;
}

/// Built-in slash commands the session socket answers directly, without running a
/// turn. Listed alongside skills in the "/" popup so they're discoverable rather
/// than folklore.
export const CONTEXT_COMMANDS: { name: string; description: string }[] = [
  { name: "context", description: "How much of the context window this session is using" },
  { name: "compact", description: "Fold older turns into a summary to free up context" },
  { name: "undo", description: "Put back what my last self-learning pass changed" },
];

/// "@" targets — things you address a message TO, as opposed to "/" commands, which
/// are things you ask the session to do.
///
/// The fixed ones. The other assistants join them at runtime (`useMentions`): the
/// mention was always a shape (`@connector`, `@persona`) that the brain was the
/// first of. `label` is what a row also matches on besides its handle — a name.
///
/// Like "/", this is discoverability only: the backend parses `@brain` and `@<id>` out
/// of the message text itself (`core::brain::parse_mention`, `core::coworker::mentioned`),
/// so typing either by hand works identically and nothing here has to be kept in step
/// with a wire field.
export type Mention = { name: string; description: string; label?: string };
export const MENTIONS: Mention[] = [
  {
    name: "brain",
    description: "Ask everything I know about you — memories, timeline, knowledge and profile",
  },
];
