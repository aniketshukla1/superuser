import type { ReactNode } from "react";
import type { ProviderField } from "../../api";
import { openExternal } from "../../tauri";
import { AuthMethodPanel } from "./AuthMethodPanel";
import { CustomEndpoint } from "./CustomEndpoint";
import { KEY_HELP, ProviderMark, type ProviderSetupState } from "./ProviderSetupState";

const LABEL = "block mb-1 mt-3 text-muted text-sm";
const INPUT = "bg-panel border focus:border-accent outline-none px-3 py-2 rounded-lg text-md w-full";
const STATUS_BADGE = "-translate-y-1/2 absolute bg-okSoft font-medium pointer-events-none px-2 py-0.5 right-2 rounded-full text-ok text-xs top-1/2";
const LINK = "decoration-line hover:text-ink text-muted underline underline-offset-2";

/** Whether every condition in a field's `show_when` holds for the current values. */
const shownFor = (field: ProviderField, values: Record<string, string>) =>
  Boolean(field.show_when) && Object.entries(field.show_when!).every(([key, wanted]) => (values[key] || "") === wanted);

/** Where to get a key, or for a keyless provider how to install it. */
function KeyHelp({ ps, sel }: { ps: ProviderSetupState; sel: string }) {
  const info = ps.info;
  if (!info) return null;
  if (!info.needs_key) {
    return (
      <p className="mt-2 text-faint text-xs">
        No API key needed — Ollama runs models on this computer.{" "}
        <button className={LINK} onClick={() => openExternal("https://ollama.com/download")}>
          Install Ollama ↗
        </button>
      </p>
    );
  }
  const help = KEY_HELP[sel];
  if (!help) return null;
  return (
    <p className="mt-2 text-faint text-xs">
      No key yet?{" "}
      <button className={LINK} onClick={() => openExternal(help.url)}>
        Create one at {help.label} ↗
      </button>{" "}
      — takes about a minute.
    </p>
  );
}

/** One provider's key form: the way back, the brand heading, the fields (the endpoint behind a
 *  quiet disclosure), the saved pill inside the field, Test or Detect, help for getting a key,
 *  and an error line of fixed height. `footer` comes after the error line (Settings puts
 *  "Remove key…" there). */
export function ProviderForm({ ps, tp, footer }: { ps: ProviderSetupState; tp: string; footer?: ReactNode }) {
  const { info, sel } = ps;
  const all = info?.fields ?? [];
  const keyed = all.some((field) => field.secret);
  // A cloud provider declares a segmented choice of sign-in method; the chosen method's fields
  // go inside a card with its own Test & save.
  const choice = all.find((field) => (field.choices?.length ?? 0) > 0);
  const method = choice ? ps.fields[choice.key] || choice.default || "" : "";
  const methodFields = choice ? all.filter((field) => shownFor(field, ps.fields)) : [];
  // Without a choice, Test sits beside the required secret (the API key), or beside the first
  // field of a keyless provider (Ollama's Detect).
  const testKey = all.find((field) => field.secret && field.required)?.key ?? all[0]?.key;
  if (!sel) return null;

  const testing = ps.verify.state === "testing";
  const fieldRow = (field: ProviderField, testable: boolean) => {
    const showSaved = ps.savedState && testable;
    return (
      <div key={field.key}>
        <label className={LABEL}>{field.label}</label>
        <div className="flex gap-2">
          <div className="flex-1 min-w-0 relative">
            <input
              className={`${INPUT} ${showSaved ? "border-ok pr-32" : "border-line"}`}
              type={field.secret ? "password" : "text"}
              placeholder={field.secret && ps.credentialed && !ps.dirty ? "••••••••" : field.placeholder}
              value={ps.fields[field.key] || ""}
              data-testid={`${tp}-field-${field.key}`}
              onChange={(e) => ps.setFieldValue(field.key, e.target.value)}
              onBlur={field.secret ? undefined : () => void ps.saveField(field.key)}
            />
            {ps.fieldSaved === field.key && (
              <span className={STATUS_BADGE} data-testid={`${tp}-field-saved-${field.key}`}>
                ✓ Saved
              </span>
            )}
            {/* §39: the state is shown IN the field, never as lines underneath. */}
            {showSaved && (
              <span className={STATUS_BADGE} data-testid={`${tp}-saved-pill`}>
                {info?.needs_key ? <>✓ Tested &amp; saved</> : <>✓ Detected</>}
              </span>
            )}
          </div>
          {testable && (
            <button
              className="border border-line disabled:opacity-40 font-medium hover:border-lineStrong px-4 rounded-lg shrink-0 text-ink text-md"
              onClick={() => ps.runTestAndSave()}
              disabled={testing || (!ps.secretFilled && !ps.credentialed)}
              data-testid={`${tp}-test`}
            >
              {testing ? "…" : info?.needs_key ? "Test" : "Detect"}
            </button>
          )}
        </div>
        {field.help && <p className="mt-1 text-faint text-xs">{field.help}</p>}
      </div>
    );
  };

  // Plain fields: not part of a method, not the choice itself, and not the endpoint of a keyed provider.
  const plain = all.filter((field) => !field.show_when && !(field.choices?.length) && !(keyed && field.key === "base_url"));

  return (
    <div>
      <button className="hover:text-ink text-muted text-sm" onClick={ps.backToGallery} data-testid={`${tp}-back`}>
        ‹ All providers
      </button>
      <div className="flex gap-3 items-center mb-1 mt-3">
        <ProviderMark name={info?.name || ""} title={info?.title || ""} size={36} />
        <span className="min-w-0">
          <span className="block font-semibold leading-tight text-lg">{info?.title}</span>
          {info ? ps.statusFor(info) : null}
        </span>
      </div>
      {info?.blurb && <p className="mt-1 text-faint text-xs">{info.blurb}</p>}
      {plain.map((field) => fieldRow(field, !choice && field.key === testKey))}
      {choice && <AuthMethodPanel ps={ps} tp={tp} choice={choice} method={method} methodFields={methodFields} fieldRow={fieldRow} />}
      <KeyHelp ps={ps} sel={sel} />
      <CustomEndpoint ps={ps} tp={tp} />
      {/* The error line keeps its height, so a failure never shifts the form. */}
      <div className="min-h-[19px] mt-3 text-sm">{ps.verify.state === "error" && <span className="text-warnInk">{ps.verify.msg}</span>}</div>
      {footer}
    </div>
  );
}
