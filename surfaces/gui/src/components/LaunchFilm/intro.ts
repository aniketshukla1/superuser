// Which cut of the launch film plays, and the steps its bottom bar reads out.

export interface BootStep {
  label: string;
  /** true = done, false = not yet, "failed" = we know it won't. */
  state: boolean | "failed";
}

/** The full film on a first launch, a five-second cut after, the last frame only when off. */
export type IntroMode = "full" | "short" | "off";

export const INTRO_PREF = "superuser.intro";
export const INTRO_SEEN = "superuser.intro.seen";

/** Which cut to play this launch, and remember that the intro has been seen. Storage
 *  failing (private mode, blocked site data) gets the short cut rather than the whole film
 *  on every launch. Reduced motion, or the intro switched off, gets the last frame only. */
export function pickIntro(): IntroMode {
  try {
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return "off";
    if (localStorage.getItem(INTRO_PREF) === "off") return "off";
    const seen = localStorage.getItem(INTRO_SEEN);
    localStorage.setItem(INTRO_SEEN, String(Date.now()));
    return seen ? "short" : "full";
  } catch {
    return "short";
  }
}

