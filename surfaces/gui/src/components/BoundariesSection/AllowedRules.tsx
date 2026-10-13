import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";
import { CODE_INPUT } from "./INPUT";

/** Allowed: checked after the deny rules; one expression per line. */
export function AllowedRules({
  allowText,
  allow,
  problemFor,
  onChange,
}: {
  allowText: string;
  allow: string[];
  problemFor: (i: number) => string | null;
  onChange: (text: string) => void;
}) {
  return (
    <details className={CARD + " p-4 mb-4"}>
      <summary className={FIELD_LABEL + " cursor-pointer"}>Allowed</summary>
      <div className={FIELD_HELP}>
        Checked after the rules above. <code>true</code> permits anything they don&rsquo;t refuse. An empty list
        refuses everything, which is a way to stop every action at once.
      </div>
      <textarea
        className={CODE_INPUT + " mt-2.5"}
        rows={Math.max(2, allowText.split("\n").length + 1)}
        spellCheck={false}
        aria-label="Allowed rules, one per line"
        data-testid="boundaries-allow"
        value={allowText}
        onChange={(e) => onChange(e.target.value)}
      />
      {allow.map((expr, i) =>
        problemFor(i) ? (
          <div key={i} className="text-sm text-danger mt-1.5">
            <code>{expr}</code>: {problemFor(i)}
          </div>
        ) : null,
      )}
    </details>
  );
}
