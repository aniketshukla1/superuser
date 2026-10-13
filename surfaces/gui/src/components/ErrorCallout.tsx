// A failure, rendered as a failure.
//
// It used to be one of two things: a centred line of accent-coloured text ("Error: model
// unreachable"), or — for a memory question — an ordinary answer bubble with a provider's
// JSON pasted into it, thumbs and all. Neither said what to do next. This says what failed
// in a heading, why in one sentence, and offers the next step: try again, go to where the
// fix is, or read the reason exactly as it was written.
//
// Deliberately quiet — a card with a danger mark, not a red slab. An old conversation can
// hold a dozen of these, and a transcript that shouts at every one of them is unreadable.
import { useState } from "react";
import type { Failure } from "../failure";
import { Icon, ICON } from "./Icon";

export function ErrorCallout({
  failure,
  onRetry,
  onFix,
}: {
  failure: Failure;
  /** Offered only where retrying means something — the transcript tail of an idle session. */
  onRetry?: () => void;
  /** Where the fix lives. Omitted when the host cannot navigate. */
  onFix?: (where: NonNullable<Failure["fix"]>) => void;
}) {
  const [open, setOpen] = useState(false);
  const hasDetail = !!failure.detail && failure.detail !== failure.summary;
  const fix = failure.fix && onFix ? failure.fix : undefined;
  return (
    <div className="error-callout" data-testid="error-callout">
      <Icon name="alert" size={ICON.md} className="error-callout-icon" />
      <div className="error-callout-body">
        <div className="error-callout-title">{failure.title}</div>
        <p className="error-callout-summary">{failure.summary}</p>
        {(onRetry || fix || hasDetail) && (
          <div className="error-callout-actions">
            {onRetry && (
              <button type="button" className="btn sm" data-testid="notice-retry" onClick={onRetry}>
                Retry
              </button>
            )}
            {fix === "models" && (
              <button type="button" className="btn sm" data-testid="error-fix" onClick={() => onFix?.("models")}>
                Open Models settings
              </button>
            )}
            {hasDetail && (
              <button
                type="button"
                className="btn sm ghost"
                aria-expanded={open}
                data-testid="error-details"
                onClick={() => setOpen((o) => !o)}
              >
                {open ? "Hide details" : "Details"}
              </button>
            )}
          </div>
        )}
        {open && hasDetail && <pre className="error-callout-detail">{failure.detail}</pre>}
      </div>
    </div>
  );
}
