import type { Attachment } from "../../types";
import { AttachChip } from "./SlashRow";

/** Above the box: a dictation or voice error, a rejected-attachment notice, and the
 * attachments about to be sent. */
export function ComposerNotices({
  error,
  attachNotice,
  onDismissNotice,
  attachments,
  onRemove,
}: {
  error?: string | null;
  attachNotice: string | null;
  onDismissNotice: () => void;
  attachments: Attachment[];
  onRemove: (i: number) => void;
}) {
  return (
    <>
      {error && (
        <div className="max-w-3xl mx-auto mb-2 px-1 text-sm text-red-600" role="alert">
          {error}
        </div>
      )}

      {/* Rejected-attachment notice (PDF over the user's Token-savings thresholds). */}
      {attachNotice && (
        <div
          data-testid="attach-notice"
          className="max-w-3xl mx-auto mb-1.5 flex items-center gap-2 rounded-lg border border-warnInk/30 bg-warnSoft px-3 py-1.5 text-sm text-warnInk"
        >
          <span className="flex-1">{attachNotice}</span>
          <button
            className="shrink-0 opacity-60 hover:opacity-100"
            onClick={onDismissNotice}
            title="Dismiss"
          >
            ✕
          </button>
        </div>
      )}

      {/* Attachments preview — a strip ABOVE the input box (mock/Claude-style). */}
      {attachments.length > 0 && (
        <div className="max-w-3xl mx-auto mb-1.5 flex flex-wrap gap-2">
          {attachments.map((a, i) => (
            <AttachChip key={i} a={a} onRemove={() => onRemove(i)} />
          ))}
        </div>
      )}
    </>
  );
}
