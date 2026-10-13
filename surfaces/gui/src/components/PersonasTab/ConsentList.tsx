import type { PersonaConsent } from "../../api";
import { CARD } from "../../ui";

/** What each newly installed assistant can do, shown before it is enabled. */
export function ConsentList({ consent }: { consent: PersonaConsent[] }) {
  if (consent.length === 0) return null;
  return (
    <div className="mt-4 space-y-2">
      {consent.map((c) => (
        <div key={c.id} className={CARD + " p-3.5"}>
          <div className="text-md font-medium">{c.name}</div>
          <div className="text-sm text-muted mt-0.5 mb-2">{c.description}</div>
          <div className="text-sm text-ink">Tools: {c.tools.join(", ") || "—"}</div>
          <div className="text-sm text-ink">
            Risk: {c.risk.join(", ") || "read"}
            {c.connectors ? " · connectors" : ""}
            {c.messaging ? " · messaging" : ""}
            {c.mcp.length ? ` · mcp: ${c.mcp.join(", ")}` : ""}
          </div>
          <div className="text-sm text-faint mt-1">
            Recommended mode: {c.recommended_mode}
            {c.needs_capable_model ? " · needs a capable model" : ""}. Enable it above to
            use it.
          </div>
          {/* Where it came from, kept on the card because months later "who wrote
              this" is the question, and the manifest on disk is the only record. */}
          {c.source && (
            <div className="text-sm text-faint mt-0.5 truncate" title={c.source}>
              From {c.source}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
