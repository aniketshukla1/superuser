import { CONNECTORS, FALLBACK, type ConnectorRegistryEntry } from "./CONNECTORS";

/**
 * Resolve a logo id to its registry entry plus the matched key. Unknown / empty ids return the
 * FALLBACK entry with key `"fallback"` (so callers and tests can distinguish a hit from a miss).
 */
export function resolveConnector(logo?: string): { key: string; entry: ConnectorRegistryEntry } {
  const id = (logo ?? "").trim();
  if (id && Object.prototype.hasOwnProperty.call(CONNECTORS, id)) {
    return { key: id, entry: CONNECTORS[id] };
  }
  return { key: "fallback", entry: FALLBACK };
}
