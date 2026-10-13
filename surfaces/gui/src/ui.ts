// Shared class-name constants.
//
// These were declared sixteen times across ten files for four ideas: `CARD` nine
// times, `SEC_H` four, `FIELD_LABEL` twice. Not carelessness — the same failure the
// type scale had, one reasonable local decision at a time with nowhere to look the
// answer up. And it had already produced its first divergence: GalleryModal's card
// is `rounded-xl bg-panel/60` where every other is `rounded-xl2 bg-panel`, which
// may be deliberate and is impossible to tell from the code.
//
// The values themselves are Tailwind utilities pointing at the --r-* and --t-*
// tokens; this file is about there being ONE name for each, not a second scale.

/** A panel card: the standard container for a settings or list section. */
export const CARD = "rounded-xl2 border border-line bg-panel";

/** A section heading inside a card or page — "About", "Built-in capabilities". */
export const SEC_H = "text-xs text-faint font-semibold";

/** The label on a settings row. */
export const FIELD_LABEL = "text-sm font-medium text-ink";

/** The explanatory line under a label. Prose line-height, since it wraps. */
export const FIELD_HELP = "text-sm text-muted mt-1.5 leading-prose";
