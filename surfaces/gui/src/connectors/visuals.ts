// Turning a connector or recommendation ref (an id such as "github", or an MCP server's name)
// into what ConnectorIcon and ConnectorBadge draw: {logo, brand_color, label}.
//
// The brand colour belongs to the API's connector descriptor (`brand_color`), so the list from
// `/v1/connectors` is indexed by id and each descriptor's `logo` and colour pass straight
// through; nothing here keeps a colour table of its own. A ref with no descriptor is tried as
// a logo id (an MCP ref gets the MCP glyph) in a neutral colour, like the registry's fallback.

import type { Connector } from "../api";
import type { ConnectorVisual } from "./ConnectorIcon";

export type ConnectorMap = Record<string, Connector>;

/** The `/v1/connectors` list keyed by connector id (`name`). */
export function indexConnectors(list: Connector[]): ConnectorMap {
  return Object.fromEntries(list.map((c) => [c.name, c]));
}

/** A bare ref with a capital first letter, for when no descriptor has a real title. */
export function humanize(ref: string): string {
  return ref ? ref[0].toUpperCase() + ref.slice(1) : "";
}

/** The descriptor's title when there is one, otherwise the capitalised id. */
export function labelFor(ref: string, byName: ConnectorMap): string {
  return byName[ref]?.title || humanize(ref);
}

/** What to draw for a ref: a known connector's own logo and colour, the MCP glyph for an MCP
 *  ref, or else the ref tried as a logo id, which ends at the neutral plug when unknown. */
export function visualFor(ref: string, kind: string, byName: ConnectorMap): ConnectorVisual {
  const known = byName[ref];
  if (known) return { logo: known.logo || ref, brand_color: known.brand_color, title: known.title, label: known.title };
  return { logo: kind === "mcp" ? "mcp" : ref, label: humanize(ref) };
}
