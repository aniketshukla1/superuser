import type { BoundariesState } from "../../api";
import { CARD, FIELD_HELP, FIELD_LABEL, SEC_H } from "../../ui";
import { EXAMPLES } from "./INPUT";

/** What a rule can read: the operators, examples, and every field. */
export function RuleReference({ fields }: { fields: BoundariesState["fields"] }) {
  return (
    <details className={CARD + " p-4 mb-4"}>
      <summary className={FIELD_LABEL + " cursor-pointer"}>What a rule can read</summary>
      <div className={FIELD_HELP}>
        Rules use <code>==</code>, <code>!=</code>, <code>&&</code>, <code>||</code>, <code>!</code>, <code>in [ ]</code>, and{" "}
        <code>contains</code>, <code>startsWith</code>, <code>endsWith</code>, <code>matches</code> — which ignore capitals.
      </div>
      <div className={SEC_H + " mt-3 mb-1.5"}>Examples</div>
      {EXAMPLES.map((ex) => (
        <div key={ex.when} className="py-1 text-sm">
          <code className="text-ink">{ex.when}</code> <span className="text-muted">— {ex.says}</span>
        </div>
      ))}
      <div className={SEC_H + " mt-3 mb-1.5"}>Fields</div>
      <div className="divide-y divide-line">
        {fields.map((f) => (
          <div key={f.field} className="py-1.5 flex items-baseline gap-3 text-sm">
            <code className="text-ink shrink-0 w-32">{f.field}</code>
            <span className="text-muted min-w-0">{f.means}</span>
          </div>
        ))}
      </div>
    </details>
  );
}
