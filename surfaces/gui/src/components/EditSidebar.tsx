import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Icon, ICON } from "./Icon";
import { RAIL_NAV, type AppSurface } from "../nav";

/**
 * Choosing which nav rows are on screen.
 *
 * The rows behind "More" used to be a fixed set decided in nav.ts — a reasonable default,
 * but a default is not the same as a choice, and which surfaces someone reaches for daily is
 * not something this code can know. This makes it theirs.
 *
 * Applied live rather than on Done: the sidebar is visible behind the dialog, so a checkbox
 * that does nothing until you dismiss the thing you are looking at makes you close it to
 * find out what you did. Done therefore closes, and there is no Cancel — nothing here is
 * destructive and everything is one click to put back.
 */
export function EditSidebar(props: {
  visible: AppSurface[];
  onChange: (ids: AppSurface[]) => void;
  onClose: () => void;
}) {
  const doneRef = useRef<HTMLButtonElement>(null);
  const [shown, setShown] = useState<Set<AppSurface>>(() => new Set(props.visible));

  // Focus comes in, and goes back where it came from on the way out. It used to land on the
  // page body: Escape closed the dialog and a keyboard was left at the top of the document,
  // several tab stops from the row it had opened this with.
  const openedFrom = useRef<HTMLElement | null>(
    typeof document === "undefined" ? null : (document.activeElement as HTMLElement | null),
  );
  useEffect(() => {
    doneRef.current?.focus();
    return () => openedFrom.current?.focus?.();
  }, []);

  const toggle = (id: AppSurface) => {
    const next = new Set(shown);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setShown(next);
    // Order comes from RAIL_NAV, not from click order — otherwise unchecking and
    // rechecking something moves it to the bottom of the sidebar.
    props.onChange(RAIL_NAV.filter((n) => next.has(n.id)).map((n) => n.id));
  };

  return createPortal(
    <div
      className="fixed inset-0 z-50"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          props.onClose();
        }
      }}
      role="dialog"
      aria-modal="true"
      aria-label="Edit sidebar"
      data-testid="edit-sidebar"
    >
      <div className="absolute inset-0 bg-black/40 backdrop-blur-[1px]" onClick={props.onClose} />
      <div className="absolute left-1/2 top-[20vh] -translate-x-1/2 w-[400px] max-w-[92vw] rounded-xl2 border border-line bg-panel shadow-2xl">
        <div className="px-5 pt-5 pb-2 flex items-start">
          <div className="min-w-0 flex-1">
            <h2 className="text-lg font-semibold text-ink leading-tight">Edit sidebar</h2>
            <p className="text-sm text-muted mt-1">
              Choose which items appear in your sidebar.
            </p>
          </div>
          <button
            className="w-7 h-7 -mt-1 -mr-1.5 grid place-items-center rounded-row text-faint hover:text-ink hover:bg-solid shrink-0"
            aria-label="Close"
            onClick={props.onClose}
          >
            <Icon name="x" size={ICON.md} />
          </button>
        </div>

        <div className="px-5 py-3 space-y-0.5">
          {RAIL_NAV.map((n) => {
            const on = shown.has(n.id);
            return (
              // A real <input type=checkbox>, visually hidden and driving a drawn box.
              // A div with role=checkbox would have to reimplement Space, the focus ring
              // and the form semantics, and would still not be announced as reliably.
              <label
                key={n.id}
                className="flex items-center gap-3 py-1.5 px-1 -mx-1 rounded-row cursor-pointer hover:bg-solid"
                data-testid={`edit-sidebar-${n.id}`}
              >
                <input
                  type="checkbox"
                  className="peer sr-only"
                  checked={on}
                  onChange={() => toggle(n.id)}
                />
                <span
                  className={
                    "w-[18px] h-[18px] rounded-card grid place-items-center shrink-0 border " +
                    "peer-focus-visible:ring-2 peer-focus-visible:ring-accent peer-focus-visible:ring-offset-1 peer-focus-visible:ring-offset-panel " +
                    (on ? "bg-accent border-accent text-onAccent" : "border-lineStrong text-transparent")
                  }
                  aria-hidden
                >
                  <Icon name="check" size={ICON.sm} />
                </span>
                <Icon name={n.icon} size={ICON.md} className="shrink-0 text-ink" />
                <span className="text-md text-ink">{n.label}</span>
              </label>
            );
          })}
        </div>

        <div className="px-5 pb-5 pt-1 flex justify-end">
          <button
            ref={doneRef}
            className="btn-primary"
            onClick={props.onClose}
          >
            Done
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
