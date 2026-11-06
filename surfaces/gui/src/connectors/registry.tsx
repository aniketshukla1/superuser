// The connector logo registry: a stable `logo` id from the API descriptor to the brand's
// one-colour mark and a label. Colours are not kept here; they come from the descriptor's
// `brand_color`. An unknown or empty id gets FALLBACK, a neutral plug.
//
// Brand marks are 24×24 single paths that paint with `currentColor`, so ConnectorIcon and
// ConnectorBadge tint them with the connector's colour. Brands without a published
// one-colour mark, and the utilities (email, browser, MCP, the plug), use simple line glyphs
// drawn for this app. The file is `.tsx` because the entries are JSX.

export { CONNECTORS, FALLBACK } from "./registry/CONNECTORS";
export type { ConnectorRegistryEntry } from "./registry/CONNECTORS";
export { resolveConnector } from "./registry/resolveConnector";
