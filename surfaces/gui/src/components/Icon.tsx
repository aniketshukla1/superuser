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

export { ICON, AVATAR } from "./Icon/IconName";
export type { IconName } from "./Icon/IconName";
export { Icon } from "./Icon/Icon";
