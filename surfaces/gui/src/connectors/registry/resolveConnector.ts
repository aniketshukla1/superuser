import { CONNECTORS, FALLBACK, type ConnectorRegistryEntry } from "./CONNECTORS";

/** The registry entry for a logo id, with the key it matched. An unknown or empty id gives
 *  FALLBACK under the key "fallback", so callers and tests can tell a miss from a hit. */
export function resolveConnector(logo?: string): { key: string; entry: ConnectorRegistryEntry } {
  const id = logo?.trim() ?? "";
  const known = id !== "" && Object.prototype.hasOwnProperty.call(CONNECTORS, id);
  return known ? { key: id, entry: CONNECTORS[id] } : { key: "fallback", entry: FALLBACK };
}
