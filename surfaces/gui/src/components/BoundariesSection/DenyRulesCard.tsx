import type { Boundaries, BoundaryPreset } from "../../api";
import { CARD, FIELD_HELP, FIELD_LABEL, SEC_H } from "../../ui";
import { BTN_BORDERED, CODE_INPUT, INPUT } from "./INPUT";

/** Never: the deny rules, each with what checking found or what its preset costs, and the presets. */
export function DenyRulesCard({
  deny,
  presets,
  added,
  messageFor,
  presetById,
  setRule,
  removeRule,
  addBlank,
  addPreset,
}: {
  deny: Boundaries["deny"];
  presets: BoundaryPreset[];
  added: Set<string | undefined>;
  messageFor: (i: number) => string | null;
  presetById: (id?: string) => BoundaryPreset | undefined;
  setRule: (i: number, patch: { name?: string; when?: string }) => void;
  removeRule: (i: number) => void;
  addBlank: () => void;
  addPreset: (p: BoundaryPreset) => void;
}) {
  return (
    <div className={CARD + " p-4 mb-4"} data-testid="boundaries-deny">
      <div className={FIELD_LABEL}>Never</div>
      <div className={FIELD_HELP}>
        A rule that can&rsquo;t be read counts as a match, so a typo refuses rather than quietly allowing. Boundaries only
        take permission away — your approval settings still ask as they do now.
      </div>

      {deny.length === 0 ? (
        <div className="text-sm text-muted mt-3">No boundaries yet — nothing is refused. Add a preset below or write your own.</div>
      ) : (
        <div className="mt-3 divide-y divide-line">
          {deny.map((rule, i) => {
            const preset = presetById(rule.preset);
            const message = messageFor(i);
            return (
              <div key={i} className="py-3" data-testid="boundary-rule">
                <div className="flex items-center gap-2">
                  <input
                    className={INPUT}
                    placeholder="Name it, e.g. Never send messages"
                    value={rule.name ?? ""}
                    aria-label="Rule name"
                    onChange={(e) => setRule(i, { name: e.target.value })}
                  />
                  <button className="text-sm text-danger px-2 py-1 shrink-0" onClick={() => removeRule(i)}>
                    Remove
                  </button>
                </div>
                <input
                  className={CODE_INPUT + " mt-2"}
                  placeholder={`intent == "send"`}
                  value={rule.when}
                  spellCheck={false}
                  autoComplete="off"
                  aria-label="Rule"
                  data-testid="boundary-rule-when"
                  onChange={(e) => setRule(i, { when: e.target.value })}
                />
                {message ? (
                  <div className="text-sm text-danger mt-1.5" data-testid="boundary-rule-problem">
                    {message}
                  </div>
                ) : preset ? (
                  <div className="text-sm text-muted mt-1.5" data-testid="boundary-rule-cost">
                    <span className="font-medium text-ink">What it costs:</span> {preset.cost}
                  </div>
                ) : null}
              </div>
            );
          })}
        </div>
      )}

      <div className="mt-3">
        <button className={BTN_BORDERED} onClick={addBlank} data-testid="boundary-add-blank">
          Write a rule
        </button>
      </div>

      <div className={SEC_H + " mt-5 mb-2"}>Presets</div>
      <div className="divide-y divide-line" data-testid="boundary-presets">
        {presets.map((p) => (
          <div key={p.id} className="py-2.5 flex items-start gap-3">
            <div className="min-w-0 flex-1">
              <div className="text-md text-ink">{p.name}</div>
              <div className="text-sm text-muted mt-0.5 leading-prose">{p.cost}</div>
            </div>
            <button
              className={BTN_BORDERED}
              disabled={added.has(p.id)}
              onClick={() => addPreset(p)}
              data-testid={`boundary-preset-${p.id}`}
            >
              {added.has(p.id) ? "Added" : "Add"}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
