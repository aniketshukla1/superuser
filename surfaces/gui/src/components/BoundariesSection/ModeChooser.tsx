import type { Boundaries } from "../../api";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";

/** When a rule matches: stop the action, or record it and let it through. */
export function ModeChooser({ mode, onMode }: { mode: Boundaries["mode"]; onMode: (m: Boundaries["mode"]) => void }) {
  return (
    <div className={CARD + " p-4 mb-4"}>
      <div className={FIELD_LABEL}>When a rule matches</div>
      <div className="seg mt-2.5" role="radiogroup" aria-label="When a rule matches">
        <button
          className={mode === "enforce" ? "active" : ""}
          data-testid="boundaries-mode-enforce"
          onClick={() => onMode("enforce")}
        >
          Stop the action
        </button>
        <button
          className={mode === "record" ? "active" : ""}
          data-testid="boundaries-mode-record"
          onClick={() => onMode("record")}
        >
          Record it and let it through
        </button>
      </div>
      <div className={FIELD_HELP}>
        {mode === "enforce"
          ? "A refused action doesn't happen, and the assistant is told which boundary stopped it."
          : "Nothing is stopped. Each action a rule would refuse is written to Privacy ▸ Actions, so you can watch a rule against real work before it refuses anything."}
      </div>
    </div>
  );
}
