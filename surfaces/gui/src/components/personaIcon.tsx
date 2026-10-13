// Persona icon resolution (one source of truth for the sidebar, persona page, and sources drawer).
//
// A persona's `icon` (from the manifest, or the built-in registry) resolves to, in order:
//   1. an emoji        — rendered as-is (e.g. a manifest with `icon: 🛠️`),
//   2. a named glyph    — a value from our Icon set (e.g. `icon: wrench`),
//   3. a legacy logo-id — the built-ins' historic ids (cowork/code/chat/ops),
//   4. the family icon  — the fallback when a manifest declares nothing usable.
// So personas are visually distinct (Ops→wrench, Code→code, Cowork→diamond) instead of all ◆.

import { Icon, type IconName } from "./Icon";

const LEGACY: Record<string, IconName> = {
  cowork: "diamond",
  chat: "chat",
  code: "code",
  ops: "wrench",
};

// The glyphs a persona manifest may name directly (a curated, persona-appropriate subset).
const NAMED: ReadonlySet<string> = new Set<IconName>([
  "sparkle",
  "diamond",
  "chat",
  "code",
  "wrench",
  "sliders",
  "search",
  "clock",
  "folder",
  "shield",
  "table",
  "plug",
  "audit",
  "branch",
  "pencil",
  // A check mark for the assistant whose job is verifying — added when the QA
  // Analyst shipped and the manifest test caught that this curated subset had no
  // glyph for it. The set is curated, not exhaustive, so growing it is expected;
  // silently falling through to the family glyph is what is not.
  "check",
  // Added with the Finance Analyst, HR Partner and Customer Support assistants.
  "coins",
  "users",
  "helpCircle",
  // Added with the Email Assistant, the email bot's drafter.
  "inbox",
  // And the Expense Manager's, and the Chief of Staff's.
  "receipt",
  "clipboard",
  // And Bug Reproduction's, and Product Performance's.
  "bug",
  "gauge",
]);

export function isEmojiIcon(icon?: string): boolean {
  return !!icon && /[^\x00-\x7F]/.test(icon);
}

export function personaGlyph(icon?: string, family?: string): IconName {
  if (icon && NAMED.has(icon)) return icon as IconName;
  if (icon && LEGACY[icon]) return LEGACY[icon];
  return family === "code" ? "code" : "sparkle";
}

/** Renders a persona's icon: an emoji as text, otherwise the resolved line glyph. */
export function PersonaGlyph({
  icon,
  family,
  size = 14,
}: {
  icon?: string;
  family?: string;
  size?: number;
}) {
  if (isEmojiIcon(icon)) {
    return <span style={{ fontSize: size, lineHeight: 1 }}>{icon}</span>;
  }
  return <Icon name={personaGlyph(icon, family)} size={size} />;
}
