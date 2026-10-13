/** The durable instructions file. Same path the agent reads at the top of each turn. */
export const AGENTS = "AGENTS.md";

/** A "+ Add" that asks rather than opening a form. Chips send; so does the free option. */
export function GuidedAdd({
  question,
  chips,
  freeform,
  onSend,
  onCancel,
}: {
  question: string;
  chips: string[];
  freeform: string;
  onSend: (text: string) => void;
  onCancel: () => void;
}) {
  return (
    <div className="asst-guided">
      <div className="asst-guided-q">{question}</div>
      <div className="asst-chips">
        {chips.map((c) => (
          <button key={c} className="asst-chip" onClick={() => onSend(c)}>
            {c}
          </button>
        ))}
      </div>
      <div className="asst-guided-foot">
        <button className="asst-link" onClick={() => onSend(freeform)}>
          Describe it in the chat instead
        </button>
        <button className="asst-link muted" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
}

export function Section({
  title,
  action,
  onAction,
  children,
}: {
  title: string;
  action?: string;
  onAction?: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="asst-section">
      <div className="asst-section-head">
        <span className="asst-section-title">{title}</span>
        {action && (
          <button className="asst-link" onClick={onAction}>
            {action}
          </button>
        )}
      </div>
      {children}
    </div>
  );
}
