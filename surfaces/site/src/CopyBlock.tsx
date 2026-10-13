import { useRef, useState } from "react";

// A command with a copy button. When the clipboard is refused (some embedded browsers do),
// the text is selected instead and the button says what to press, so the copy still
// takes one keystroke.
export function CopyBlock({ label, code }: { label: string; code: string }) {
  const [state, setState] = useState<"idle" | "copied" | "selected">("idle");
  const pre = useRef<HTMLPreElement>(null);
  const timer = useRef<number>();

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setState("copied");
    } catch {
      const range = document.createRange();
      if (pre.current) range.selectNodeContents(pre.current);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(range);
      setState("selected");
    }
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setState("idle"), 1800);
  };

  return (
    <div className="cmd">
      <div className="cmd-head">
        <span className="cmd-label mono">{label}</span>
        <button type="button" className="cmd-copy" onClick={copy} aria-label={`Copy: ${label}`}>
          {state === "copied" ? "Copied" : state === "selected" ? "Press ⌘C" : "Copy"}
        </button>
      </div>
      <pre ref={pre}>
        <code>{code}</code>
      </pre>
    </div>
  );
}
