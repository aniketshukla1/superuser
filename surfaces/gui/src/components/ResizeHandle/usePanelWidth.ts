import { useCallback, useEffect, useState } from "react";

export const PANEL_DRAG_EVENT = "su-panel-drag";

/** The narrowest the conversation is allowed to get.
 *
 * A panel's ceiling used to be a fraction of the WINDOW, which says nothing about what
 * is left over. At the 880px window minimum the right panel's 0.72 came to 634px, and
 * after a 224px sidebar the chat had 22 pixels. The window was within its own limits
 * the whole time; nothing was left for the thing the window is for.
 *
 * Mirrored as `--chat-min` in styles.css, and a test asserts the two agree.
 */
export const CHAT_MIN_W = 420;

/** The width a conversation needs to read comfortably, as opposed to the width it can
 *  survive at.
 *
 *  CHAT_MIN_W is a floor for a drag: someone who pulls the panel wide is choosing a narrow
 *  chat, and may. It was also, by accident, the only number the DEFAULT layout consulted —
 *  so at the 1100px window the app opens at, a 560px panel was fitted beside the nav and
 *  the chat got exactly its floor: 420px, with the message text at 351px while the panel
 *  said "Nothing recorded for this conversation yet." A larger window gave a narrower chat
 *  than the 880px minimum did, because at 880 the panel hid itself.
 *
 *  This is what the panel has to leave before it opens on its own. 600 holds the
 *  transcript's 68ch measure at the prose size with the gutter rail beside it. */
export const CHAT_COMFORT_W = 600;

export function clamp(v: number, min: number, max: number) {
  return Math.min(max, Math.max(min, v));
}

/**
 * Width state for one panel, persisted under `key`.
 *
 * The ceiling is a fraction of the window, so a panel dragged wide on a large
 * display doesn't come back as an unusable slab on a laptop. That ceiling is
 * DERIVED on every render rather than held in state: stored bounds go stale the
 * moment the window resizes without a re-render, which is how a panel ended up
 * narrower than its own minimum.
 */
/** The live window width. Exported because the joint clamp in App needs the same number
 *  the panels are clamping against, and a second listener that could disagree with this
 *  one is worse than one they share. */
export function useWindowWidth() {
  const [w, setW] = useState(() => window.innerWidth);
  useEffect(() => {
    const onResize = () => setW(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  return w;
}

/** Squeeze a panel so the conversation keeps `CHAT_MIN_W`, given what the other panel is
 *  taking right now.
 *
 *  Separate from the hook, and applied at RENDER rather than on commit, because the two
 *  panels constrain each other and something has to break the cycle. Doing it here means
 *  the stored preference survives: drag the panel wide on a large display, come back on a
 *  laptop, and it renders narrow — widen the window and it is wide again. Clamping on
 *  commit instead would have thrown the preference away the first time it did not fit.
 *
 *  Never below `min`: when the window genuinely cannot hold all three, making this panel
 *  unusable does not give the conversation a usable amount either.
 */
export function fitBeside(width: number, { winW, other, min }: { winW: number; other: number; min: number }) {
  return Math.max(min, Math.min(width, winW - other - CHAT_MIN_W));
}

export function usePanelWidth(
  key: string,
  {
    fallback,
    min,
    maxFraction,
    reserve = 0,
  }: {
    fallback: number;
    min: number;
    maxFraction: number;
    /** Pixels this panel must leave for everything else — the other panel, plus the
     *  conversation's own minimum. Passed live, because it changes when the other panel
     *  is resized or hidden. */
    reserve?: number;
  },
) {
  const [stored, setStored] = useState<number>(() => {
    try {
      const raw = localStorage.getItem(key);
      const n = raw ? Number(raw) : NaN;
      return Number.isFinite(n) ? n : fallback;
    } catch {
      return fallback;
    }
  });
  const [winW, setWinW] = useState(() => window.innerWidth);

  useEffect(() => {
    const onResize = () => setWinW(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  // Two ceilings, and the lower one wins. The fraction keeps a panel from becoming an
  // unusable slab on a big display; the reserve keeps it from eating the conversation on
  // a small one. Floored at `min` last, deliberately: when the window genuinely cannot
  // hold all three at their minimums, the panel stops at its OWN minimum rather than
  // collapsing — that is a window-size problem, and making this panel unusable too
  // would not fix it. The drag simply cannot make it worse than that.
  const max = Math.max(min, Math.min(Math.round(winW * maxFraction), winW - reserve));
  const width = clamp(stored, min, max);

  const commit = useCallback(
    (next: number) => {
      const w = clamp(Math.round(next), min, max);
      setStored(w);
      try {
        localStorage.setItem(key, String(w));
      } catch {
        /* best effort */
      }
    },
    [key, min, max],
  );

  return [width, commit] as const;
}
