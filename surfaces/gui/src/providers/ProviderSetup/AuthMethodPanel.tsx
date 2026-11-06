import type { ReactNode } from "react";
import type { ProviderField } from "../../api";
import type { ProviderSetupState } from "./ProviderSetupState";

type Choice = NonNullable<ProviderField["choices"]>[number];

const SEGMENT = "flex gap-1.5 items-center px-3 py-1.5 rounded-lg text-sm transition-colors whitespace-nowrap";
const SEGMENT_ON = "bg-panel font-medium ring-1 ring-line shadow-sm text-ink";
const SEGMENT_OFF = "hover:text-ink text-muted";

/** The joined track of sign-in methods. */
function MethodTrack({ ps, tp, choice, method }: { ps: ProviderSetupState; tp: string; choice: ProviderField; method: string }) {
  return (
    <div className="bg-line/40 border border-line gap-0.5 inline-flex p-0.5 rounded-xl2" role="radiogroup" aria-label={choice.label}>
      {(choice.choices ?? []).map((option) => {
        const on = option.value === method;
        return (
          <button
            key={option.value}
            role="radio"
            aria-checked={on}
            className={`${SEGMENT} ${on ? SEGMENT_ON : SEGMENT_OFF}`}
            data-testid={`${tp}-choice-${choice.key}-${option.value}`}
            onClick={() => ps.setFieldValue(choice.key, option.value)}
          >
            {option.label}
            {option.tag && <span className="bg-accentSoft font-semibold px-1.5 py-px rounded-full text-accent text-xs">{option.tag}</span>}
          </button>
        );
      })}
    </div>
  );
}

/** A terminal command shown as a button that copies it. */
function CopyCommand({ tp, command }: { tp: string; command: string }) {
  return (
    <button
      className="bg-panel border border-line font-mono gap-2 hover:border-lineStrong inline-flex items-center mt-2.5 px-2.5 py-1.5 rounded-lg text-ink text-sm"
      onClick={() => void navigator.clipboard?.writeText(command)}
      title="Copy command"
      data-testid={`${tp}-cmd-copy`}
    >
      {command}
      <span className="font-sans text-faint text-xs">⧉</span>
    </button>
  );
}

/** The card's footer: whether it is saved, and Test & save. */
function TestFooter({ ps, tp }: { ps: ProviderSetupState; tp: string }) {
  const testing = ps.verify.state === "testing";
  return (
    <div className="border-line border-t flex gap-3 items-center justify-between mt-3.5 pt-3">
      {ps.savedState ? (
        <span className="font-medium text-ok text-xs" data-testid={`${tp}-saved-pill`}>
          ✓ Tested &amp; saved
        </span>
      ) : (
        <span className="text-faint text-xs">Runs one read-only check, then saves.</span>
      )}
      <button
        className="bg-accent border border-accent disabled:opacity-40 font-medium hover:brightness-105 px-4 py-1.5 rounded-lg shrink-0 text-md text-white"
        onClick={() => ps.runTestAndSave()}
        disabled={testing}
        data-testid={`${tp}-test`}
      >
        {testing ? "…" : <>Test &amp; save</>}
      </button>
    </div>
  );
}

type Props = {
  ps: ProviderSetupState;
  tp: string;
  choice: ProviderField;
  method: string;
  methodFields: ProviderField[];
  fieldRow: (f: ProviderField, testable: boolean) => ReactNode;
};

/** How to sign in to a provider (owner, 2026-07-26): one joined track of methods, then a soft
 *  inset card with only the chosen method's description, fields and its own Test & save. */
export function AuthMethodPanel({ ps, tp, choice, method, methodFields, fieldRow }: Props) {
  const picked: Choice | undefined = choice.choices?.find((option) => option.value === method);
  return (
    <div>
      <label className="block mb-1 mt-3 text-muted text-sm">{choice.label}</label>
      <MethodTrack ps={ps} tp={tp} choice={choice} method={method} />
      <div className="bg-paper/60 border border-line mt-2.5 pb-3.5 pt-3 px-4 rounded-xl">
        {picked?.desc && <p className="text-muted text-sm">{picked.desc}</p>}
        {picked?.command && <CopyCommand tp={tp} command={picked.command} />}
        {methodFields.map((field) => fieldRow(field, false))}
        <TestFooter ps={ps} tp={tp} />
      </div>
    </div>
  );
}
