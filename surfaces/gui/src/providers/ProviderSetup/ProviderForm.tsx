import type { ReactNode } from "react";
import type { ProviderField as ProviderFieldT } from "../../api";
import { openExternal } from "../../tauri";
import { AuthMethodPanel } from "./AuthMethodPanel";
import { CustomEndpoint } from "./CustomEndpoint";
import { KEY_HELP, ProviderMark, type ProviderSetupState } from "./ProviderSetupState";

/** One provider's key form: crumb, brand head, fields (endpoint behind a quiet
 * disclosure), in-field saved pill, Test/Detect, key help, fixed error line.
 * `footer` renders after the error line (Settings adds "Remove key…" there). */
export function ProviderForm({
  ps,
  tp,
  footer,
}: {
  ps: ProviderSetupState;
  tp: string;
  footer?: ReactNode;
}) {
  const { info, sel } = ps;
  const label = "block text-sm text-muted mt-3 mb-1";
  const input =
    "w-full px-3 py-2 rounded-lg border bg-panel text-md outline-none focus:border-accent";
  const fieldsAll = info?.fields || [];
  const keyed = fieldsAll.some((x) => x.secret);
  // Cloud providers declare a segmented auth-method choice; the selected method's
  // credential fields render inside a panel with its own Test & save footer.
  const choice = fieldsAll.find((f) => f.choices && f.choices.length);
  const method = choice ? ps.fields[choice.key] || choice.default || "" : "";
  const methodFields = choice
    ? fieldsAll.filter(
        (f) =>
          f.show_when &&
          Object.entries(f.show_when).every(([k, v]) => (ps.fields[k] || "") === v),
      )
    : [];
  // Without a choice control, Test lives next to the required secret (the API key), or
  // the first field for keyless providers (Ollama's Detect).
  const requiredSecret = fieldsAll.find((x) => x.secret && x.required);
  const testKey = requiredSecret ? requiredSecret.key : fieldsAll[0]?.key;
  if (!sel) return null;

  const fieldRow = (f: ProviderFieldT, testable: boolean) => (
    <div key={f.key}>
      <label className={label}>{f.label}</label>
      <div className="flex gap-2">
        <div className="relative flex-1 min-w-0">
          <input
            className={input + (ps.savedState && testable ? " border-ok pr-32" : " border-line")}
            type={f.secret ? "password" : "text"}
            placeholder={f.secret && ps.credentialed && !ps.dirty ? "••••••••" : f.placeholder}
            value={ps.fields[f.key] || ""}
            data-testid={`${tp}-field-${f.key}`}
            onChange={(e) => ps.setFieldValue(f.key, e.target.value)}
            onBlur={f.secret ? undefined : () => void ps.saveField(f.key)}
          />
          {ps.fieldSaved === f.key && (
            <span
              className="absolute right-2 top-1/2 -translate-y-1/2 text-xs font-medium text-ok bg-okSoft rounded-full px-2 py-0.5 pointer-events-none"
              data-testid={`${tp}-field-saved-${f.key}`}
            >
              ✓ Saved
            </span>
          )}
          {/* §39: state lives IN the field — no status lines below. */}
          {ps.savedState && testable && (
            <span
              className="absolute right-2 top-1/2 -translate-y-1/2 text-xs font-medium text-ok bg-okSoft rounded-full px-2 py-0.5 pointer-events-none"
              data-testid={`${tp}-saved-pill`}
            >
              {info?.needs_key ? <>✓ Tested &amp; saved</> : <>✓ Detected</>}
            </span>
          )}
        </div>
        {testable && (
          <button
            className="px-4 rounded-lg border border-line text-md font-medium text-ink hover:border-lineStrong shrink-0 disabled:opacity-40"
            onClick={() => ps.runTestAndSave()}
            disabled={ps.verify.state === "testing" || (!ps.secretFilled && !ps.credentialed)}
            data-testid={`${tp}-test`}
          >
            {ps.verify.state === "testing" ? "…" : info?.needs_key ? "Test" : "Detect"}
          </button>
        )}
      </div>
      {f.help && <p className="text-xs text-faint mt-1">{f.help}</p>}
    </div>
  );

  return (
    <div>
      <button className="text-sm text-muted hover:text-ink" onClick={ps.backToGallery} data-testid={`${tp}-back`}>
        ‹ All providers
      </button>
      <div className="flex items-center gap-3 mt-3 mb-1">
        <ProviderMark name={info?.name || ""} title={info?.title || ""} size={36} />
        <span className="min-w-0">
          <span className="block text-lg font-semibold leading-tight">{info?.title}</span>
          {info ? ps.statusFor(info) : null}
        </span>
      </div>
      {info?.blurb && <p className="text-xs text-faint mt-1">{info.blurb}</p>}

      {fieldsAll
        .filter(
          (f) =>
            !f.show_when &&
            !(f.choices && f.choices.length) &&
            !(f.key === "base_url" && keyed),
        )
        .map((f) => fieldRow(f, !choice && f.key === testKey))}

      {choice && (
        <AuthMethodPanel ps={ps} tp={tp} choice={choice} method={method} methodFields={methodFields} fieldRow={fieldRow} />
      )}

      {info?.needs_key && KEY_HELP[sel] && (
        <p className="text-xs text-faint mt-2">
          No key yet?{" "}
          <button
            className="text-muted underline decoration-line underline-offset-2 hover:text-ink"
            onClick={() => openExternal(KEY_HELP[sel].url)}
          >
            Create one at {KEY_HELP[sel].label} ↗
          </button>{" "}
          — takes about a minute.
        </p>
      )}
      {info && !info.needs_key && (
        <p className="text-xs text-faint mt-2">
          No API key needed — Ollama runs models on this computer.{" "}
          <button
            className="text-muted underline decoration-line underline-offset-2 hover:text-ink"
            onClick={() => openExternal("https://ollama.com/download")}
          >
            Install Ollama ↗
          </button>
        </p>
      )}

      <CustomEndpoint ps={ps} tp={tp} />

      {/* Error line: fixed height so failures never reflow the form. */}
      <div className="mt-3 min-h-[19px] text-sm">
        {ps.verify.state === "error" && <span className="text-warnInk">{ps.verify.msg}</span>}
      </div>
      {footer}
    </div>
  );
}
