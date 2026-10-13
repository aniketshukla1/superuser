import type { Connector } from "../../api";
import { GRP, ROW, TAG_QUIET, TAG_WARN } from "./ui";

// Collapsed-by-default Tools group, shared by every connector detail page
// (UX-DECISIONS §21): quiet until expanded, then one row per assistant tool that
// reaches this connector, with its read/write tag; writes always ask first. A list,
// not switches — see ConnectorTools for why there is no per-connector off.
export function ToolsDisclosure({ c }: { c: Connector }) {
  if (!c.tools?.length) return null;
  const n = c.tools.length;
  return (
    <div className={GRP + " mt-6"}>
      <details>
        <summary className={ROW + " cursor-pointer hover:bg-paper/60 list-none [&::-webkit-details-marker]:hidden"}>
          <span className="text-sm text-muted w-24 shrink-0">› Tools</span>
          <span className="min-w-0 flex-1 text-sm text-muted">
            {n} {n === 1 ? "tool" : "tools"} assistants can use
          </span>
        </summary>
        {c.tools.map((tool) => (
          <div key={tool.name} className={ROW} title={`${tool.name} — ${tool.description}`}>
            <span className="min-w-0 flex-1 text-md font-medium">{tool.label}</span>
            <span className={tool.kind === "write" ? TAG_WARN : TAG_QUIET}>
              {tool.kind === "write" ? "asks first" : "read"}
            </span>
          </div>
        ))}
      </details>
    </div>
  );
}
