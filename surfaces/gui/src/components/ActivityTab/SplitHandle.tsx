import { useCallback, useRef, useState } from "react";
import { PANEL_DRAG_EVENT } from "../ResizeHandle";

/* ---- The split ------------------------------------------------------------ */

export const SPLIT_KEY = "assistant:activity-split:v1";
/** Percent of the tab's height given to the top (plan + calls) half. */
export const SPLIT_FALLBACK = 50;
export const SPLIT_MIN = 15;
export const SPLIT_MAX = 85;

/** Keep either half from collapsing to a sliver you can neither read nor grab.
 *  Exported because the clamping is the part worth pinning down in a test. */
export function clampSplit(pct: number): number {
  if (!Number.isFinite(pct)) return SPLIT_FALLBACK;
  return Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, Math.round(pct)));
}

export function useSplit() {
  const [pct, setPct] = useState<number>(() => {
    try {
      const raw = localStorage.getItem(SPLIT_KEY);
      const n = raw ? Number(raw) : NaN;
      return Number.isFinite(n) ? clampSplit(n) : SPLIT_FALLBACK;
    } catch {
      return SPLIT_FALLBACK;
    }
  });
  const commit = useCallback((next: number) => {
    const v = clampSplit(next);
    setPct(v);
    try {
      localStorage.setItem(SPLIT_KEY, String(v));
    } catch {
      /* best effort */
    }
  }, []);
  return [pct, commit] as const;
}

/**
 * The divider. Measured against the tab's own box rather than the window, since
 * this split is interior — and, like the side handles, it listens on `window` so a
 * cursor that outruns a 6px strip doesn't drop the drag. It broadcasts the same
 * `su-panel-drag` the side handles do, so a native child webview steps aside for
 * the duration instead of swallowing the pointer.
 */
export function SplitHandle({
  boxRef,
  pct,
  onResize,
}: {
  boxRef: React.RefObject<HTMLDivElement>;
  pct: number;
  onResize: (next: number) => void;
}) {
  const latest = useRef(onResize);
  latest.current = onResize;

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    const box = boxRef.current;
    if (!box) return;
    const rect = box.getBoundingClientRect();
    if (rect.height < 1) return;
    const broadcast = (active: boolean) =>
      window.dispatchEvent(new CustomEvent(PANEL_DRAG_EVENT, { detail: { active } }));

    const move = (ev: PointerEvent) => latest.current(((ev.clientY - rect.top) / rect.height) * 100);
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      document.body.classList.remove("resizing-row");
      broadcast(false);
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    document.body.classList.add("resizing-row");
    broadcast(true);
  };

  return (
    <div
      className="act-split"
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize raw output"
      aria-valuenow={pct}
      tabIndex={0}
      data-testid="act-split"
      onPointerDown={onPointerDown}
      onDoubleClick={() => onResize(SPLIT_FALLBACK)}
      // Keyboard resize — a pointer-only affordance is unreachable for anyone who
      // can't hold a drag.
      onKeyDown={(e) => {
        const step = e.shiftKey ? 12 : 4;
        if (e.key === "ArrowUp") {
          e.preventDefault();
          onResize(pct - step);
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          onResize(pct + step);
        }
      }}
    />
  );
}
