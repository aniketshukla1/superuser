import type { Transition, Variants } from "framer-motion";

// One motion vocabulary for the whole site, so everything shares a rhythm rather
// than each section inventing its own timing.
//
// The tiers and easings come from the design data's motion presets, which are
// written as GSAP; these are the Framer equivalents:
//
//   expo.out       -> EXPO      — long, hard deceleration. Headlines.
//   power1.out     -> DECELERATE— short, gentle. Lists, copy, most things.
//   back.out(1.4)  -> OVERSHOOT — settles past its mark. Cards only.
//
// Two rules from that data are load-bearing and easy to lose:
//
//  1. Overshoot is for cards and grids, never for informational rows. On a dense
//     list it reads as sloppy — the content is being presented, not celebrated.
//  2. Character-level splitting is for headlines under ~8 words. Splitting a
//     paragraph makes one DOM node per character and the accessibility tree loses
//     the sentence.

export const EXPO = [0.16, 1, 0.3, 1] as const;
export const DECELERATE = [0.25, 0.46, 0.45, 0.94] as const;
export const OVERSHOOT = [0.34, 1.56, 0.64, 1] as const;

/** Headline characters. 600ms/expo, 15ms apart — the "complex" tier. */
export const charRise: Variants = {
  hidden: { opacity: 0, y: 20, rotateX: -40 },
  shown: (i: number) => ({
    opacity: 1,
    y: 0,
    rotateX: 0,
    transition: { duration: 0.6, ease: EXPO, delay: i * 0.015 },
  }),
};

/** Anything that is read rather than admired. 300ms/decelerate. */
export const rise: Variants = {
  hidden: { opacity: 0, y: 8 },
  shown: { opacity: 1, y: 0, transition: { duration: 0.3, ease: DECELERATE } },
};

/** Cards and grid cells. 400ms with the overshoot — see rule 1 above. */
export const cardRise: Variants = {
  hidden: { opacity: 0, y: 16, scale: 0.97 },
  shown: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.4, ease: OVERSHOOT } },
};

/**
 * Stagger a group's children.
 *
 * `each` defaults to 60ms, the grid figure. Rows want ~30ms — a long list at 60ms
 * takes longer to finish arriving than a reader takes to reach the bottom of it.
 */
export function stagger(each = 0.06, delay = 0): Variants {
  return {
    hidden: {},
    shown: { transition: { staggerChildren: each, delayChildren: delay } },
  };
}

/**
 * Scroll-reveal props. `once` so sections don't re-animate on the way back up.
 *
 * `amount` is deliberately a sliver rather than a quarter. These variants animate
 * opacity from 0, which means an element whose trigger never fires is not merely
 * un-animated — it is INVISIBLE, permanently. On a page whose entire job is to be
 * read, the reveal must be almost impossible to miss, so one pixel of the element
 * entering the viewport is enough.
 *
 * A quarter was the default and it was too strict: a tall section on a short
 * window, a fast scroll, a deep link landing past it, or a full-page screenshot
 * (which resizes the viewport and repaints) could all leave a section blank. Two
 * different capture passes each dropped a different section that way.
 */
export const onScroll = {
  initial: "hidden",
  whileInView: "shown",
  viewport: { once: true, amount: 0.01 },
} as const;

/**
 * The reduced-motion answer for every variant above: render the final state, with
 * no transform and no duration.
 *
 * Deliberately not a shortened duration. A 10ms slide is still a slide, and the
 * setting is a request to not be moved rather than to be moved briefly.
 */
export const still: Variants = { hidden: { opacity: 1 }, shown: { opacity: 1 } };
export const NO_TRANSITION: Transition = { duration: 0 };
