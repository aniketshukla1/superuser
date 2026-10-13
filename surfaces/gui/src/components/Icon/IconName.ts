// A small set of clean, single-weight line icons (SF-Symbols-ish): 24px grid, 1.7 stroke,
// currentColor, rounded caps/joins. Replaces emoji in the chrome for a crisp, consistent look.

/** How big an icon is allowed to be.
 *
 * Eleven sizes were in use across 136 call sites — 13, 14, 15 and 16 accounted for 114 of
 * them, four adjacent values doing one job. Nobody chose four; each was chosen once, next
 * to whatever it sat beside, and the next person matched the nearest neighbour.
 *
 * Four steps, named for where they go. `size` still takes a number, so this is a
 * convention rather than a type — the guard in design.system.test.ts is what makes it a
 * rule.
 */
export const ICON = {
  /** Inline with small text: status glyphs, marks inside a badge. */
  sm: 12,
  /** The default. Rows, chips, menus, anything in a 24px control. */
  md: 15,
  /** Prominent: settings rows, list leaders, a control you are meant to notice. */
  lg: 18,
  /** One-offs at panel scale, where the icon IS the affordance. */
  xl: 28,
} as const;

/** How big a LOGO TILE is allowed to be.
 *
 * Separate from ICON on purpose. A connector badge at 44px is not an icon that grew; it
 * is a different thing — a brand tile, square, with the logo inset — and forcing it onto
 * a scale that tops out at 18 would have meant either shrinking every avatar or inventing
 * icon steps nothing else uses. ConnectorBadge alone had nine sizes across fifteen files.
 */
export const AVATAR = {
  /** Beside a line of text: a row leader, a chip. */
  sm: 20,
  /** In a list or a card header. */
  md: 26,
  /** A detail page's subject. */
  lg: 34,
  /** The one you are being asked to choose. */
  xl: 44,
} as const;

export type IconName =
  | "sparkle"
  | "logo"
  | "sidebar"
  | "sidebarRight"
  | "signOut"
  | "chat"
  | "diamond"
  | "book"
  | "search"
  | "folder"
  | "folderPlus"
  | "plus"
  | "plusCircle"
  | "home"
  | "check"
  | "speaker"
  | "dots"
  | "clock"
  | "sliders"
  | "gear"
  | "inbox"
  | "code"
  | "wrench"
  | "pencil"
  | "branch"
  | "arrowLeft"
  | "arrowRight"
  | "copy"
  | "refresh"
  | "panelClose"
  | "panelOpen"
  | "plug"
  | "audit"
  | "chevronRight"
  | "chevronDown"
  | "moreHorizontal"
  | "pin"
  | "archive"
  | "trash"
  | "shield"
  | "file"
  | "fileCode"
  | "image"
  | "table"
  | "mic"
  | "paperclip"
  | "helpCircle"
  | "users"
  | "coins"
  | "receipt"
  | "clipboard"
  | "bug"
  | "gauge"
  | "stop"
  | "globe"
  | "alert"
  | "thumbUp"
  | "thumbDown"
  | "x";
