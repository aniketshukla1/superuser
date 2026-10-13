import { useState } from "react";
import { setSmallModel, setEmbedModel, type ModelSettings } from "../../api";
import { SEC_H } from "../../ui";

/**
 * Which local model does the work you aren't waiting on.
 *
 * Triage, summaries, conversation titles and the nightly consolidation all used
 * the chat model, because it was the only local one there was. On a machine
 * running a 7B+ model that means the background loops compete for the GPU with
 * the conversation they exist to serve. Only local models are offered: the point
 * is to spend less, and a background loop on a metered API is the opposite.
 */
/**
 * What memory search embeds with.
 *
 * This had no control at all, which is how a chat model ended up in the slot and
 * stayed there: Ollama answers `501` for anything that can't embed, recall quietly
 * falls back to keyword matching, and the only route back was hand-editing
 * settings.json. A setting the whole memory system rests on has to be reachable
 * from the app.
 *
 * Only models that can actually embed are offered, and the server refuses the rest
 * a second time — the picker is a convenience, not the guard.
 */
export function EmbeddingModelCard({
  settings,
  onChanged,
}: {
  settings: ModelSettings;
  onChanged: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const current = settings.embed_model || "";
  // An older server sends neither field; offering an empty picker would be worse
  // than showing nothing.
  if (settings.embed_model === undefined) return null;
  const installed = settings.embed_models ?? [];
  // Tag-insensitively, the way the server matches: the setting is stored untagged
  // (`nomic-embed-text`) while Ollama lists it tagged (`nomic-embed-text:latest`),
  // and comparing them literally reports a perfectly good configuration as broken.
  // `base_name` in core/ollama.rs is the same rule on the other side.
  const bare = (m: string) => m.split(":")[0];
  const have = installed.some((m) => bare(m) === bare(current));
  // The configured model may not be installed, or may not be an embedding model at
  // all — it still has to appear, or the picker would silently misreport what is
  // actually in use.
  const options = current && !have ? [current, ...installed] : installed;
  const broken = current !== "" && !have;
  // Show the installed entry that is actually serving, tag and all. The setting is
  // stored untagged, so binding the select to it directly leaves the value matching
  // no option, and the browser silently falls back to the first one — a control
  // displaying a choice nobody made.
  const shown = installed.find((m) => bare(m) === bare(current)) ?? current;
  return (
    <div className="mt-6" data-testid="embed-model">
      <div className={SEC_H + " mb-1.5"}>Memory search</div>
      <p className="text-sm text-muted mb-2.5 leading-prose">
        The model your memories are embedded with, so search can find them by meaning rather
        than by matching words. It must be an embedding model — a chat model here still answers,
        but only with keyword matching, and nothing says so.
      </p>
      {options.length === 0 ? (
        <p className="text-sm text-muted">
          No embedding model installed. Run <code>ollama pull nomic-embed-text</code>, then reopen
          this page.
        </p>
      ) : (
        <select
          className="px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
          data-testid="embed-model-select"
          value={shown}
          onChange={(e) => {
            setError(null);
            void setEmbedModel(e.target.value).then((r) => (r.ok ? onChanged() : setError(r.error || "couldn't save")));
          }}
        >
          {options.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      )}
      {broken && (
        <p className="text-sm text-danger mt-2" data-testid="embed-model-warning">
          “{current}” isn’t an installed embedding model — memory search is falling back to keyword
          matching.
        </p>
      )}
      {error && <p className="text-sm text-danger mt-2">{error}</p>}
    </div>
  );
}

export function BackgroundModelCard({
  settings,
  onChanged,
}: {
  settings: ModelSettings;
  onChanged: () => void;
}) {
  const local = settings.models.filter((m) => !m.includes(":") || m.split(":")[1]?.match(/^\d/));
  const current = settings.small_model || "";
  return (
    <div className="mt-6" data-testid="background-model">
      <div className={SEC_H + " mb-1.5"}>Background model</div>
      <p className="text-sm text-muted mb-2.5 leading-prose">
        Used for work you aren't waiting on — triaging what changed, summarizing, naming
        conversations. Leave it as the chat model unless that one is large; a small model here
        keeps the background loops off the weights you're talking to. The nightly memory passes
        always use the chat model, because they need to hold a strict format.
      </p>
      <select
        className="px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
        data-testid="background-model-select"
        value={current}
        onChange={(e) => setSmallModel(e.target.value).then((r) => r.ok && onChanged())}
      >
        <option value="">Same as the chat model{settings.model ? ` (${settings.model})` : ""}</option>
        {local
          .filter((m) => m !== settings.model)
          .map((m) => (
            <option key={m} value={m}>
              {settings.model_labels?.[m] || m}
            </option>
          ))}
      </select>
    </div>
  );
}

// Curated OAuth quick-adds: remote MCP servers with browser sign-in (OAuth 2.1 + DCR) —
// no keys to paste, tokens stay in the local secret store. First: Granola.
export const MCP_PRESETS: { name: string; label: string; blurb: string; config: Record<string, any> }[] = [
  {
    name: "granola",
    label: "Granola",
    blurb: "Meeting notes & transcripts — sign in with your Granola account.",
    config: { type: "http", url: "https://mcp.granola.ai/mcp", auth: "oauth" },
  },
];
