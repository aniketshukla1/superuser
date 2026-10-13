import type { Connector, SessionConnections } from "../../api";
import { ConnectorBadge } from "../../connectors/ConnectorIcon";
import { visualFor, type ConnectorMap } from "../../connectors/visuals";
import { Icon, ICON, AVATAR } from "../Icon";

/** §32 addendum (owner ask 2026-07-13; FB-012): the catalog's long tail, in-session. A quiet
 * row that becomes a typeahead: full list on focus, filter as you type. */
export function AddSourcePicker({
  adding,
  setAdding,
  query,
  setQuery,
  byName,
  connected,
  onPick,
}: {
  adding: boolean;
  setAdding: (on: boolean) => void;
  query: string;
  setQuery: (q: string) => void;
  byName: ConnectorMap;
  connected: SessionConnections["connected"];
  onPick: (c: Connector) => void;
}) {
  // Catalog list: available, not already in the Connected list (those have toggles above).
  // Empty query = the whole catalog (FB-012 — the list renders before any typing); a query
  // narrows it on title/name/aliases ("calendar" must surface Outlook, not just Google
  // Calendar). Alphabetical so filtering never reorders; the container height-caps it, so
  // no count cap here.
  const connectedSet = new Set(connected.map((c) => c.connector));
  const q = query.trim().toLowerCase();
  const results = Object.values(byName)
    .filter(
      (c) =>
        c.available &&
        !connectedSet.has(c.name) &&
        (!q ||
          c.title.toLowerCase().includes(q) ||
          c.name.toLowerCase().includes(q) ||
          (c.aliases ?? []).some((a) => a.toLowerCase().includes(q))),
    )
    .sort((a, b) => a.title.localeCompare(b.title));

  return (
    <>
      {adding ? (
        <div className="mt-1.5">
          <input
            className="w-full px-2.5 py-1.5 rounded-lg border border-line bg-panel text-sm outline-none focus:border-accent"
            placeholder="Search connectors…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setAdding(false);
                setQuery("");
              }
            }}
            autoFocus
            data-testid="access-add-search"
          />
          {results.length === 0 && (
            // Also covers a failed/empty catalog fetch: an open picker must never
            // be silently blank — point at the Connectors page either way.
            <div className="text-xs text-faint mt-1.5 px-0.5">
              No match — see all on the Connectors page below.
            </div>
          )}
          <div className="mt-1 max-h-64 overflow-y-auto">
            {results.map((c) => (
              <button
                key={c.name}
                className="w-full flex items-center gap-2 py-1.5 px-0.5 rounded-lg text-left hover:bg-paper"
                data-testid={`access-add-${c.name}`}
                onClick={() => {
                  setAdding(false);
                  setQuery("");
                  onPick(c);
                }}
              >
                <ConnectorBadge connector={visualFor(c.name, "connector", byName)} size={AVATAR.sm} />
                <span className="min-w-0 flex-1">
                  <span className="block text-sm font-medium leading-tight">
                    {c.title}
                  </span>
                  <span className="block text-xs text-faint truncate">{c.blurb}</span>
                </span>
                <Icon name="chevronRight" size={ICON.sm} className="text-faint shrink-0" />
              </button>
            ))}
          </div>
        </div>
      ) : (
        <button
          className="mt-1 text-sm text-accent hover:underline text-left"
          onClick={() => setAdding(true)}
          data-testid="access-add-source"
        >
          + Add a source…
        </button>
      )}
    </>
  );
}
