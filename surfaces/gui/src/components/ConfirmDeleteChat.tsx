import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { getArtifacts } from "../api";
import { Icon, ICON } from "./Icon";

/**
 * Confirming the deletion of a chat.
 *
 * Deleting is `remove_dir_all` on the session's folder, and that folder holds more
 * than the transcript — every file the agent produced in that chat lives there too.
 * So this names what actually goes: the messages, and the files by name. A dialog
 * that only says "are you sure?" makes the user guess at the stakes, and the stakes
 * here are the deliverables they asked for.
 *
 * The files are fetched when the dialog opens rather than carried on the session
 * row, because the row is rendered for every chat in the sidebar and this is needed
 * for exactly one of them, once, at the moment someone is about to destroy it.
 */
export function ConfirmDeleteChat(props: {
  title: string;
  sessionId: string;
  messages: number;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const [files, setFiles] = useState<string[] | null>(null);
  // Cancel holds focus, not Delete. The dialog exists because this is irreversible,
  // so the key that fires on an unread dialog must be the one that changes nothing.
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancelRef.current?.focus();
  }, []);

  useEffect(() => {
    let alive = true;
    getArtifacts(props.sessionId)
      .then((a) => alive && setFiles(a.map((f) => f.name)))
      // A listing that fails is not a reason to block the delete, and not a reason
      // to claim there are no files either — `[]` reads as "none", so an error
      // stays `null` and the dialog simply says less.
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [props.sessionId]);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      props.onCancel();
    }
  };

  return createPortal(
    <div
      className="fixed inset-0 z-50"
      onKeyDown={onKey}
      role="dialog"
      aria-modal="true"
      aria-label="Delete chat"
      data-testid="confirm-delete-chat"
    >
      <div className="absolute inset-0 bg-black/30 backdrop-blur-[1px]" onClick={props.onCancel} />
      <div className="absolute left-1/2 top-[22vh] -translate-x-1/2 w-[420px] max-w-[92vw] rounded-xl2 border border-line bg-panel shadow-2xl overflow-hidden">
        <div className="px-4 pt-4 pb-3 flex items-start gap-3">
          <span className="w-8 h-8 rounded-full grid place-items-center bg-dangerSoft text-danger shrink-0">
            <Icon name="trash" size={ICON.md} />
          </span>
          <div className="min-w-0">
            <h2 className="text-lg font-semibold text-ink leading-tight">Delete this chat?</h2>
            {/* The title truncated rather than wrapped: a long one would push the
                buttons off the bottom of a dialog whose whole job is to be read. */}
            <p className="text-sm text-faint mt-0.5 truncate" title={props.title}>
              {props.title}
            </p>
          </div>
        </div>

        <div className="px-4 pb-3.5 text-sm text-ink space-y-2">
          <p>
            This permanently deletes{" "}
            <strong className="font-medium">
              {props.messages} {props.messages === 1 ? "message" : "message"}
              {props.messages === 1 ? "" : "s"}
            </strong>
            {files && files.length > 0 ? (
              <>
                {" "}
                and{" "}
                <strong className="font-medium">
                  {files.length} {files.length === 1 ? "file" : "files"}
                </strong>{" "}
                produced in it.
              </>
            ) : (
              <> from this chat.</>
            )}
          </p>
          {files && files.length > 0 && (
            <ul className="text-faint bg-paper border border-line rounded-lg px-2.5 py-1.5 max-h-24 overflow-y-auto hairline-scroll">
              {files.map((f) => (
                <li key={f} className="truncate leading-prose">
                  {f}
                </li>
              ))}
            </ul>
          )}
          <p className="text-faint">This cannot be undone.</p>
        </div>

        <div className="px-4 py-3 border-t border-line flex items-center justify-end gap-2">
          {/* Archive is the reversible neighbour of this action, and someone who
              opened this dialog by accident is usually reaching for it. */}
          <button
            ref={cancelRef}
            className="text-sm px-2.5 py-1.5 rounded-lg border border-line bg-paper hover:border-lineStrong shrink-0"
            data-testid="confirm-delete-cancel"
            onClick={props.onCancel}
          >
            Cancel
          </button>
          <button
            className="text-sm px-2.5 py-1.5 rounded-lg bg-danger text-white font-medium shrink-0 hover:brightness-110"
            data-testid="confirm-delete-confirm"
            onClick={props.onConfirm}
          >
            Delete chat
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
