// What importing an agent package would do, before it does it.
//
// Three lists, in the order a person needs them: what arrives, what doesn't (each with
// its reason), and what has no home here at all. The instructions are behind a
// disclosure rather than hidden — they are what will steer a model, and reading them is
// the review.

import type { PackagePlan } from "../api";
import { CARD, SEC_H } from "../ui";

const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export function PackagePlanView({
  plan,
  busy,
  onImport,
  onCancel,
}: {
  plan: PackagePlan;
  busy: boolean;
  onImport: () => void;
  onCancel: () => void;
}) {
  const people = plan.personas.filter((p) => p.problems.length === 0);
  const skills = plan.skills.filter((s) => s.problems.length === 0);
  const refused = [
    ...plan.personas.filter((p) => p.problems.length > 0).map((p) => ({ key: `p-${p.id}`, what: p.name || p.id, why: p.problems })),
    ...plan.skills.filter((s) => s.problems.length > 0).map((s) => ({ key: `s-${s.name}`, what: `Skill ${s.name}`, why: s.problems })),
  ];
  const nothing = people.length === 0 && skills.length === 0;

  return (
    <div className={CARD + " p-4 mt-4 space-y-4"} data-testid="package-plan">
      {people.length > 0 && (
        <section>
          <div className={SEC_H + " mb-1.5"}>Assistants — arrive switched off</div>
          <ul className="space-y-2">
            {people.map((p) => (
              <li key={p.id} data-testid={`plan-persona-${p.id}`}>
                <div className="text-md text-ink">
                  {p.name} <span className="text-sm text-faint">· {p.tagline}</span>
                </div>
                <div className="text-sm text-muted">
                  {p.remote ? `Runs on a remote agent at ${p.remote}` : `Tools: ${p.tools.join(", ")}`}
                  {p.skills.length > 0 && ` · Skills: ${p.skills.join(", ")}`}
                </div>
                {p.notes.map((n) => (
                  <div key={n} className="text-sm text-faint">
                    {n}
                  </div>
                ))}
                <details className="mt-1">
                  <summary className="text-sm text-muted cursor-pointer">Instructions</summary>
                  <pre className="text-sm text-ink whitespace-pre-wrap mt-1">{p.instructions}</pre>
                </details>
              </li>
            ))}
          </ul>
        </section>
      )}
      {skills.length > 0 && (
        <section>
          <div className={SEC_H + " mb-1.5"}>Skills — arrive switched off</div>
          <ul className="space-y-2">
            {skills.map((s) => (
              <li key={s.name} data-testid={`plan-skill-${s.name}`}>
                <div className="text-md text-ink">
                  {s.name} <span className="text-sm text-faint">· {s.description}</span>
                </div>
                {s.unmapped.length > 0 && (
                  <div className="text-sm text-faint">
                    Names tools from connected apps ({s.unmapped.join(", ")}) — they work here once those apps are connected.
                  </div>
                )}
                <details className="mt-1">
                  <summary className="text-sm text-muted cursor-pointer">Instructions</summary>
                  <pre className="text-sm text-ink whitespace-pre-wrap mt-1">{s.instructions}</pre>
                </details>
              </li>
            ))}
          </ul>
        </section>
      )}
      {refused.length > 0 && (
        <section>
          <div className={SEC_H + " mb-1.5"}>Not coming across</div>
          <ul className="space-y-1">
            {refused.map((r) => (
              <li key={r.key} className="text-sm text-muted" data-testid="plan-refused">
                <span className="text-ink">{r.what}:</span> {r.why.join(" ")}
              </li>
            ))}
          </ul>
        </section>
      )}
      {plan.left_behind.length > 0 && (
        <section>
          <div className={SEC_H + " mb-1.5"}>No home here</div>
          <ul className="space-y-1">
            {plan.left_behind.map((l) => (
              <li key={l} className="text-sm text-muted" data-testid="plan-left-behind">
                {l}
              </li>
            ))}
          </ul>
        </section>
      )}
      <div className="flex items-center gap-2">
        <button className={BTN_ACCENT} disabled={busy || nothing} onClick={onImport} data-testid="plan-import">
          {nothing
            ? "Nothing to import"
            : busy
              ? "Importing…"
              : `Import ${people.length} assistant${people.length === 1 ? "" : "s"} and ${skills.length} skill${skills.length === 1 ? "" : "s"}`}
        </button>
        <button className={BTN_BORDERED} disabled={busy} onClick={onCancel} data-testid="plan-cancel">
          Cancel
        </button>
      </div>
    </div>
  );
}
