import { useEffect, useState } from "react";
import { dismissNotice, subscribeNotices, type Notice } from "../notify";
import { Icon, ICON } from "./Icon";

// The icon set has no alert glyph and one notice does not justify adding one.
const AlertMark = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
    <path d="M12 9v4M12 17h.01" />
  </svg>
);

// Renders whatever `notifyError` has to say. One instance, mounted by App.
//
// `role="alert"` (not a bare div): a failed save is exactly the case where someone not
// watching this corner of the screen needs to be told (UX-REPORT §5). Bottom-left, so
// it never sits over the composer's own controls or the automation toast at top-right.

export function Notices() {
  const [notices, setNotices] = useState<Notice[]>([]);
  useEffect(() => subscribeNotices(setNotices), []);
  if (!notices.length) return null;
  return (
    <div className="fixed bottom-3 left-3 z-[60] flex flex-col gap-2 w-[320px] max-w-[80vw]">
      {notices.map((n) => (
        <div
          key={n.id}
          role="alert"
          data-testid="notice-error"
          className="rounded-xl border border-danger/40 bg-panel shadow-lg px-3.5 py-3 flex items-start gap-2.5"
        >
          <span className="text-danger shrink-0 mt-[1px]">
            <AlertMark />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block text-sm text-ink">{n.text}</span>
            {n.detail && (
              <span className="block text-xs text-muted mt-0.5 break-words">{n.detail}</span>
            )}
          </span>
          <button
            className="shrink-0 w-6 h-6 grid place-items-center rounded-md text-muted hover:text-ink hover:bg-paper"
            aria-label="Dismiss"
            data-testid="notice-dismiss"
            onClick={() => dismissNotice(n.id)}
          >
            <Icon name="x" size={ICON.md} />
          </button>
        </div>
      ))}
    </div>
  );
}
