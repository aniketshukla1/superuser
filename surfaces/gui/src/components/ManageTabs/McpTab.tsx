import { useEffect, useState } from "react";
import { addMcpServer, connectMcp, deleteMcpServer, getMcpServers, patchMcpServer, reloadMcp, type McpServer } from "../../api";
import { CARD } from "../../ui";
import { BTN_ACCENT } from "./initials";
import { MCP_PRESETS } from "./EmbeddingModelCard";
import { AddForm, McpRow } from "./McpRow";

export function McpTab() {
  const [servers, setServers] = useState<McpServer[]>([]);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => getMcpServers().then(setServers).catch(() => setServers([]));
  useEffect(() => {
    refresh();
  }, []);

  // While a browser sign-in is in flight, poll so the row flips to connected (or
  // surfaces the error) without the user having to touch anything.
  const authorizing = servers.some((s) => s.status === "authorizing");
  useEffect(() => {
    if (!authorizing) return;
    const t = window.setInterval(refresh, 2000);
    return () => window.clearInterval(t);
  }, [authorizing]);

  const toggle = async (s: McpServer) => {
    await patchMcpServer(s.name, { enabled: !s.enabled });
    refresh();
  };
  const remove = async (s: McpServer) => {
    await deleteMcpServer(s.name);
    refresh();
  };

  return (
    <div className="space-y-3">
      <p className="text-sm text-muted leading-prose">
        Enabled servers' tools are permission-gated. Changes apply to new sessions —{" "}
        <button
          className="text-accent font-medium hover:underline"
          onClick={() => reloadMcp().then(refresh)}
        >
          reload now
        </button>
        .
      </p>

      {servers.length === 0 && !adding ? (
        <div className={CARD + " p-4 text-md text-muted"}>
          No MCP servers configured.{" "}
          <button className="text-accent font-medium" onClick={() => setAdding(true)}>
            Add a server
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          {servers.map((s) => (
            <McpRow
              key={s.name}
              server={s}
              onToggle={() => toggle(s)}
              onRemove={() => remove(s)}
              onRefresh={refresh}
            />
          ))}
        </div>
      )}

      {/* One-click OAuth presets not yet configured. */}
      {MCP_PRESETS.filter((p) => !servers.some((s) => s.name === p.name)).map((p) => (
        <div key={p.name} className={CARD + " p-3.5 flex items-center gap-3"} data-testid={`mcp-preset-${p.name}`}>
          <div className="flex-1 min-w-0">
            <div className="text-lg font-medium">{p.label}</div>
            <div className="text-xs text-faint">{p.blurb}</div>
          </div>
          <button
            className={BTN_ACCENT}
            onClick={async () => {
              await addMcpServer(p.name, p.config);
              await connectMcp(p.name); // opens the browser sign-in right away
              refresh();
            }}
          >
            Connect
          </button>
        </div>
      ))}

      {adding ? (
        <AddForm
          onCancel={() => {
            setAdding(false);
            setError(null);
          }}
          onError={setError}
          onAdded={() => {
            setAdding(false);
            setError(null);
            refresh();
          }}
        />
      ) : servers.length > 0 ? (
        <button className={BTN_ACCENT} onClick={() => setAdding(true)}>
          + Add server
        </button>
      ) : null}
      {error && <div className="text-sm text-danger">{error}</div>}
    </div>
  );
}
