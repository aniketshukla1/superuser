import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import type { Attachment } from "../../types";
import { mergeAttachments, type Props } from "./Props";

/** The draft's life: the box grows with it (to four lines), it clears when the conversation
 * changes, and a prefill pushed from outside lands in it once. */
export function useDraftLifecycle({
  text,
  setText,
  setAttachments,
  setPendingSkill,
  textareaRef,
  resetKey,
  prefill,
}: {
  text: string;
  setText: (t: string) => void;
  setAttachments: (update: Attachment[] | ((cur: Attachment[]) => Attachment[])) => void;
  setPendingSkill: (s: null) => void;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  resetKey: Props["resetKey"];
  prefill: Props["prefill"];
}) {
  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    const max = parseFloat(getComputedStyle(el).lineHeight || "22") * 4;
    const next = Math.min(el.scrollHeight, max);
    el.style.height = `${Math.max(next, 24)}px`;
    el.style.overflowY = el.scrollHeight > max ? "auto" : "hidden";
  }, [text]);

  // Clear the draft when the conversation changes, so a half-typed message / picked file doesn't
  // bleed from one session into another. Declared BEFORE the prefill effect: when both fire in
  // the same render (the Skills doorway starts a new session AND prefills it), effects run in
  // declaration order — clear first, then the prefill lands on the fresh session.
  //
  // Only on a CHANGE of conversation, never on mount: a fresh composer has nothing to clear,
  // and clearing there undid the prefill. React runs mount effects twice in a dev build —
  // clear, prefill, clear again, and the second prefill is (rightly) skipped as already
  // applied — so "Create with Superuser" from Settings, which mounts the composer with its
  // prefill, opened on an empty box.
  const clearedFor = useRef(resetKey);
  useEffect(() => {
    if (clearedFor.current === resetKey) return;
    clearedFor.current = resetKey;
    setText("");
    setAttachments([]);
    setPendingSkill(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);

  // Apply a prefill (text + attachments) pushed from outside, then focus the composer. Applied at
  // most once per nonce (a ref guards against StrictMode/re-render double-fires), and attachments
  // are de-duplicated so the same file never lands twice.
  const appliedNonce = useRef<number>(-1);
  useEffect(() => {
    const p = prefill;
    if (!p || p.nonce === appliedNonce.current) return;
    appliedNonce.current = p.nonce;
    setText(p.text);
    if (p.attachments?.length) setAttachments((cur) => mergeAttachments(cur, p.attachments!));
    textareaRef.current?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [prefill?.nonce]);
}
