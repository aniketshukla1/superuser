import { useEffect, useRef, useState } from "react";
import { OTHER, type AskQuestion } from "./AskChoice";
import { Chevron, Cross } from "./Chevron";

export function AskCard({
  questions,
  onDone,
  compact,
}: {
  questions: AskQuestion[];
  /** One answer per question, in order; null where it was skipped. */
  onDone: (answers: (string | null)[]) => void;
  compact?: boolean;
}) {
  const [step, setStep] = useState(0);
  const [answers, setAnswers] = useState<(string | null)[]>(() => questions.map(() => null));
  const [picked, setPicked] = useState<string[]>([]);
  const [other, setOther] = useState("");
  const [open, setOpen] = useState(true);
  const firstOption = useRef<HTMLButtonElement>(null);

  const q = questions[step];
  const total = questions.length;
  const multi = !!q?.multi;
  const otherKey = (q?.options.length ?? 0) + 1;

  // Land focus on the first option when a question arrives, so the keyboard works
  // without hunting for the card — and so a screen reader reads the question it
  // belongs to rather than wherever focus happened to be.
  useEffect(() => {
    firstOption.current?.focus();
  }, [step]);

  if (!q) return null;

  const value = (): string | null => {
    const text = other.trim();
    if (picked.includes(OTHER)) return text || null;
    const chosen = picked.filter((p) => p !== OTHER);
    return chosen.length ? chosen.join(", ") : null;
  };
  const ready = value() !== null;

  const advance = (answer: string | null) => {
    const next = [...answers];
    next[step] = answer;
    setAnswers(next);
    setPicked([]);
    setOther("");
    if (step + 1 < total) setStep(step + 1);
    else onDone(next);
  };

  const choose = (label: string) => {
    if (multi) setPicked((p) => (p.includes(label) ? p.filter((x) => x !== label) : [...p, label]));
    else setPicked((p) => (p[0] === label ? [] : [label]));
    if (label !== OTHER) setOther("");
  };

  // Dismissing is not skipping one question — it is declining the whole ask. The
  // model is told nothing was answered, rather than handed a silence it can read
  // whichever way suits it.
  const dismiss = () => onDone(questions.map(() => null));

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") return dismiss();
    if (e.key === "Enter" && ready) return advance(value());
    const n = Number(e.key);
    if (!Number.isInteger(n) || n < 1 || n > otherKey) return;
    e.preventDefault();
    if (n === otherKey) {
      choose(OTHER);
      (e.currentTarget.querySelector(".askcard-other input") as HTMLInputElement | null)?.focus();
    } else {
      choose(q.options[n - 1].label);
    }
  };

  return (
    <div
      className={"askcard" + (compact ? " askcard-compact" : "")}
      role="group"
      aria-label={q.question}
      onKeyDown={onKey}
      data-testid="ask-card"
    >
      <div className="askcard-head">
        {total > 1 && (
          <span className="askcard-step" data-testid="ask-step">
            {step + 1}/{total}
          </span>
        )}
        <h3 className="askcard-q">{q.question}</h3>
        <button
          type="button"
          className="askcard-icon"
          aria-expanded={open}
          aria-label={open ? "Collapse" : "Expand"}
          onClick={() => setOpen((o) => !o)}
        >
          <Chevron open={open} />
        </button>
        <button
          type="button"
          className="askcard-icon"
          aria-label="Dismiss without answering"
          onClick={dismiss}
          data-testid="ask-dismiss"
        >
          <Cross />
        </button>
      </div>

      {open && (
        <>
          <div className="askcard-options">
            {q.options.map((o, i) => {
              const on = picked.includes(o.label);
              return (
                <button
                  key={o.label}
                  type="button"
                  ref={i === 0 ? firstOption : undefined}
                  className={"askcard-opt" + (on ? " is-on" : "")}
                  aria-pressed={on}
                  onClick={() => choose(o.label)}
                >
                  <span className="askcard-opt-text">
                    <span className="askcard-opt-label">{o.label}</span>
                    {o.description ? <span className="askcard-opt-desc">{o.description}</span> : null}
                  </span>
                  <kbd className="askcard-key" aria-hidden="true">
                    {i + 1}
                  </kbd>
                </button>
              );
            })}

            {/* Always present. A list of four is a guess about what you meant, and
                the escape from a wrong guess has to be there before you need it. */}
            <div
              className={"askcard-opt askcard-other" + (picked.includes(OTHER) ? " is-on" : "")}
              data-testid="ask-other"
            >
              <div className="askcard-other-head">
                <span className="askcard-opt-label">Other</span>
                <kbd className="askcard-key" aria-hidden="true">
                  {otherKey}
                </kbd>
              </div>
              <input
                aria-label={"Your own answer to: " + q.question}
                placeholder="Type your own answer here"
                value={other}
                onChange={(e) => {
                  setOther(e.target.value);
                  setPicked(e.target.value ? [OTHER] : []);
                }}
                onKeyDown={(e) => {
                  // Digits belong to the field once you are typing in it, not to the
                  // shortcuts — otherwise "Q4" selects option 4 and eats the answer.
                  e.stopPropagation();
                  if (e.key === "Enter" && other.trim()) advance(other.trim());
                  if (e.key === "Escape") dismiss();
                }}
              />
            </div>
          </div>

          <div className="askcard-foot">
            <button type="button" className="askcard-skip" onClick={() => advance(null)} data-testid="ask-skip">
              Skip
            </button>
            <button
              type="button"
              className="askcard-next"
              disabled={!ready}
              onClick={() => advance(value())}
              data-testid="ask-next"
            >
              {step + 1 < total ? "Next" : "Done"}
            </button>
          </div>
        </>
      )}
    </div>
  );
}
