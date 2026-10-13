import type { ReactNode } from "react";
import type { ProviderField } from "../../api";
import type { ProviderSetupState } from "./ProviderSetupState";

const label = "block text-sm text-muted mt-3 mb-1";

/** Auth-method segmented control + the selected method's panel (owner call 2026-07-26):
 * one joined track, then a soft inset card holding only that method's description,
 * fields, and its own Test & save footer. */
export function AuthMethodPanel({
  ps,
  tp,
  choice,
  method,
  methodFields,
  fieldRow,
}: {
  ps: ProviderSetupState;
  tp: string;
  choice: ProviderField;
  method: string;
  methodFields: ProviderField[];
  fieldRow: (f: ProviderField, testable: boolean) => ReactNode;
}) {
  const selected = choice.choices?.find((c) => c.value === method);
  return (
    <div>
      <label className={label}>{choice.label}</label>
      <div
        className="inline-flex gap-0.5 rounded-xl2 border border-line bg-line/40 p-0.5"
        role="radiogroup"
        aria-label={choice.label}
      >
        {(choice.choices || []).map((c) => {
          const active = method === c.value;
          return (
            <button
              key={c.value}
              role="radio"
              aria-checked={active}
              className={
                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm whitespace-nowrap transition-colors " +
                (active
                  ? "bg-panel text-ink font-medium shadow-sm ring-1 ring-line"
                  : "text-muted hover:text-ink")
              }
              data-testid={`${tp}-choice-${choice.key}-${c.value}`}
              onClick={() => ps.setFieldValue(choice.key, c.value)}
            >
              {c.label}
              {c.tag && (
                <span className="text-xs font-semibold text-accent bg-accentSoft rounded-full px-1.5 py-px">
                  {c.tag}
                </span>
              )}
            </button>
          );
        })}
      </div>

      <div className="mt-2.5 rounded-xl border border-line bg-paper/60 px-4 pb-3.5 pt-3">
        {selected?.desc && <p className="text-sm text-muted">{selected.desc}</p>}
        {selected?.command && (
          <button
            className="mt-2.5 inline-flex items-center gap-2 rounded-lg border border-line bg-panel px-2.5 py-1.5 text-sm font-mono text-ink hover:border-lineStrong"
            onClick={() => void navigator.clipboard?.writeText(selected.command || "")}
            title="Copy command"
            data-testid={`${tp}-cmd-copy`}
          >
            {selected.command}
            <span className="font-sans text-xs text-faint">⧉</span>
          </button>
        )}
        {methodFields.map((f) => fieldRow(f, false))}
        <div className="mt-3.5 flex items-center justify-between gap-3 border-t border-line pt-3">
          {ps.savedState ? (
            <span className="text-xs font-medium text-ok" data-testid={`${tp}-saved-pill`}>
              ✓ Tested &amp; saved
            </span>
          ) : (
            <span className="text-xs text-faint">Runs one read-only check, then saves.</span>
          )}
          <button
            className="shrink-0 rounded-lg border border-accent bg-accent px-4 py-1.5 text-md font-medium text-white hover:brightness-105 disabled:opacity-40"
            onClick={() => ps.runTestAndSave()}
            disabled={ps.verify.state === "testing"}
            data-testid={`${tp}-test`}
          >
            {ps.verify.state === "testing" ? "…" : <>Test &amp; save</>}
          </button>
        </div>
      </div>
    </div>
  );
}
