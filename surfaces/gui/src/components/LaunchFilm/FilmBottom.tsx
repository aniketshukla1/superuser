import type { BootStep } from "./intro";

/** The readings that are not staged — the real boot steps — and Skip. Nothing else: the model
 * and the version were taken off on the owner's word (2026-10-14). */
export function FilmBottom({
  steps,
  showSkip,
  onSkip,
}: {
  steps: BootStep[];
  showSkip: boolean;
  onSkip?: () => void;
}) {
  return (
    <div className="film-bottom">
      <div className="hud-steps">
        {steps.map((s, i) => (
          <span key={i} className={"hs" + (s.state === true ? " on" : s.state === "failed" ? " bad" : "")}>
            <i aria-hidden="true" />
            {s.label}
            <em>{s.state === true ? "ok" : s.state === "failed" ? "failed" : "…"}</em>
          </span>
        ))}
      </div>
      <div className="film-bottom-right">
        {showSkip && (
          <button className="film-skip" onClick={onSkip} data-testid="intro-skip">
            Skip intro <kbd>esc</kbd>
          </button>
        )}
      </div>
    </div>
  );
}
