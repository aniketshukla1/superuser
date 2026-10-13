import { useRef } from "react";
import { PANEL_DRAG_EVENT } from "./usePanelWidth";

export function ResizeHandle({
  side,
  width,
  onResize,
  label,
  defaultWidth,
}: {
  /** Which edge of the window the panel is anchored to. */
  side: "left" | "right";
  width: number;
  onResize: (next: number) => void;
  label: string;
  /** What a double-click puts the panel back to. Passed by the owner of the default rather
   *  than written here: this used to reset to 264 and 720 while the panels actually opened
   *  at 240 and 560, so "reset" produced a size the app never otherwise showed. */
  defaultWidth: number;
}) {
  // The move/up listeners go on `window`, not on the handle. Pointer capture looks
  // like the right tool but is not dependable here: the strip is 5px wide, and the
  // instant the cursor outruns it the drag dies. Window listeners can't miss.
  const latest = useRef(onResize);
  latest.current = onResize;

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    const broadcast = (active: boolean) =>
      window.dispatchEvent(new CustomEvent(PANEL_DRAG_EVENT, { detail: { active } }));

    // Measure from the window edge the panel is pinned to, so the width tracks the
    // cursor exactly rather than accumulating drift from deltas.
    const move = (ev: PointerEvent) =>
      latest.current(side === "left" ? ev.clientX : window.innerWidth - ev.clientX);
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      document.body.classList.remove("resizing-col");
      broadcast(false);
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    document.body.classList.add("resizing-col");
    broadcast(true);
  };

  return (
    <div
      className={"resize-handle " + side}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={width}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onDoubleClick={() => onResize(defaultWidth)}
      // Keyboard resize — a pointer-only affordance is unreachable for anyone
      // who can't hold a drag.
      onKeyDown={(e) => {
        const step = e.shiftKey ? 48 : 16;
        const grow = side === "left" ? "ArrowRight" : "ArrowLeft";
        const shrink = side === "left" ? "ArrowLeft" : "ArrowRight";
        if (e.key === grow) {
          e.preventDefault();
          onResize(width + step);
        } else if (e.key === shrink) {
          e.preventDefault();
          onResize(width - step);
        }
      }}
    />
  );
}
