import type { ProviderSetupState } from "./ProviderSetupState";

const LABEL = "block mb-1 mt-3 text-muted text-sm";
const INPUT = "bg-panel border border-line focus:border-accent outline-none px-3 py-2 rounded-lg text-md w-full";
const SAVED_PILL = "-translate-y-1/2 absolute bg-okSoft font-medium pointer-events-none px-2 py-0.5 right-2 rounded-full text-ok text-xs top-1/2";

/** A custom endpoint, for providers that take a key: a quiet disclosure under the key help,
 *  far enough below it to read as an advanced row of its own, with no explaining text (owner,
 *  2026-07-18 and 2026-07-19). */
export function CustomEndpoint({ ps, tp }: { ps: ProviderSetupState; tp: string }) {
  const fields = ps.info?.fields ?? [];
  const endpoint = fields.some((field) => field.secret) ? fields.find((field) => field.key === "base_url") : undefined;
  if (!endpoint) return null;
  if (!ps.showEndpoint) {
    return (
      <button className="block hover:text-ink mt-4 self-start text-muted text-sm" onClick={() => ps.setShowEndpoint(true)} data-testid={`${tp}-endpoint-link`}>
        Custom endpoint ⌄
      </button>
    );
  }
  const key = endpoint.key;
  return (
    <div className="mt-4">
      <label className={LABEL}>{endpoint.label}</label>
      <div className="relative">
        <input
          className={INPUT}
          type="text"
          placeholder={endpoint.placeholder}
          value={ps.fields[key] || ""}
          data-testid={`${tp}-field-${key}`}
          onChange={(e) => ps.setFieldValue(key, e.target.value)}
          onBlur={() => void ps.saveField(key)}
        />
        {ps.fieldSaved === key && (
          <span className={SAVED_PILL} data-testid={`${tp}-field-saved-${key}`}>
            ✓ Saved
          </span>
        )}
      </div>
      {endpoint.help && <p className="mt-1 text-faint text-xs">{endpoint.help}</p>}
    </div>
  );
}
