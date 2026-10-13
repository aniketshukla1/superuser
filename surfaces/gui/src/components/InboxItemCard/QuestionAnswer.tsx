import { useState } from "react";
import type { InboxItem } from "../../api";
import { BTN_PRIMARY, INPUT, OPT_BASE, OPT_OFF, OPT_ON } from "./KIND";

/** A parked question: its options (one click, or several and Send) and a typed answer. */
export function QuestionAnswer({
  item,
  onResolve,
}: {
  item: InboxItem;
  onResolve: (id: string, resolution: string) => void;
}) {
  const [answer, setAnswer] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const options = item.options || [];
  const multi = !!item.multi;
  const allowText = item.allow_text !== false;

  const textRow = (placeholder: string) => (
    <div className="flex items-center gap-2 mt-2.5">
      <input
        className={INPUT}
        placeholder={placeholder}
        value={answer}
        onChange={(e) => setAnswer(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && answer.trim()) onResolve(item.id, answer);
        }}
      />
      <button className={BTN_PRIMARY} disabled={!answer.trim()} onClick={() => onResolve(item.id, answer)}>
        Send
      </button>
    </div>
  );

  return (
    <>
      {options.length > 0 && (
        <div className="flex flex-wrap gap-2 mt-2.5">
          {options.map((opt) => {
            const on = selected.includes(opt);
            return (
              <button
                key={opt}
                className={OPT_BASE + " " + (on ? OPT_ON : OPT_OFF)}
                onClick={() => {
                  if (multi)
                    setSelected((s) => (on ? s.filter((x) => x !== opt) : [...s, opt]));
                  else onResolve(item.id, opt); // single-select resolves immediately
                }}
              >
                {multi && on && <span className="text-accent text-xs leading-none">✓</span>}
                {opt}
              </button>
            );
          })}
        </div>
      )}
      {multi && options.length > 0 && (
        <div className="mt-2.5">
          <button
            className={BTN_PRIMARY}
            disabled={!selected.length}
            onClick={() => onResolve(item.id, selected.join(", "))}
          >
            Send{selected.length ? ` (${selected.length})` : ""}
          </button>
        </div>
      )}
      {(allowText || options.length === 0) &&
        textRow(options.length ? "Or type your own answer…" : "Your answer…")}
    </>
  );
}
