import type { ProviderSetupState } from "./ProviderSetupState";

const label = "block text-sm text-muted mt-3 mb-1";
const input = "w-full px-3 py-2 rounded-lg border bg-panel text-md outline-none focus:border-accent";

/** Custom endpoint (keyed providers only): a quiet disclosure below the key help, with
 * enough separation to read as its own advanced row — no explainer copy (owner calls
 * 2026-07-18 + 2026-07-19). */
export function CustomEndpoint({ ps, tp }: { ps: ProviderSetupState; tp: string }) {
  const { info } = ps;
  const keyed = (info?.fields || []).some((x) => x.secret);
  const ep = keyed ? (info?.fields || []).find((f) => f.key === "base_url") : undefined;
  if (!ep) return null;
  if (!ps.showEndpoint)
    return (
      <button
        className="block self-start text-sm text-muted hover:text-ink mt-4"
        onClick={() => ps.setShowEndpoint(true)}
        data-testid={`${tp}-endpoint-link`}
      >
        Custom endpoint ⌄
      </button>
    );
  return (
    <div className="mt-4">
      <label className={label}>{ep.label}</label>
      <div className="relative">
        <input
          className={input + " border-line"}
          type="text"
          placeholder={ep.placeholder}
          value={ps.fields[ep.key] || ""}
          data-testid={`${tp}-field-${ep.key}`}
          onChange={(e) => ps.setFieldValue(ep.key, e.target.value)}
          onBlur={() => void ps.saveField(ep.key)}
        />
        {ps.fieldSaved === ep.key && (
          <span
            className="absolute right-2 top-1/2 -translate-y-1/2 text-xs font-medium text-ok bg-okSoft rounded-full px-2 py-0.5 pointer-events-none"
            data-testid={`${tp}-field-saved-${ep.key}`}
          >
            ✓ Saved
          </span>
        )}
      </div>
      {ep.help && <p className="text-xs text-faint mt-1">{ep.help}</p>}
    </div>
  );
}
