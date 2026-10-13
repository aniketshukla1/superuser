// Connector logo registry — maps a stable `logo` id (from a connector's API descriptor) to the
// brand's real monochrome mark, plus a human label. The brand color is deliberately NOT stored
// here: it comes from the API (`brand_color`) so the descriptor stays the single source of truth.
// Unknown / empty ids resolve to FALLBACK (a neutral plug glyph).
//
// Most marks come from the `simple-icons` package: official 24x24 single-path monochrome brand
// glyphs that paint with `currentColor`, so ConnectorIcon / ConnectorBadge tint them with the
// connector's brand color. Slack, Salesforce, Outlook, and Canva are no longer distributed by
// the package (removed at the trademark holders' request), so their path data is vendored below in
// the same format. Brands with no published monochrome mark (Attio, Apollo.io, Hunter,
// Amplitude, Descript, Clay, Close, Docusign — whose current post-rebrand mark no icon pack
// ships) and the non-brand utilities (email, browser, MCP, fallback plug) keep simple custom
// glyphs. (Filename is `.tsx` because the entries are JSX — the spec's `registry.ts` can't hold
// JSX.)

export { FALLBACK, CONNECTORS } from "./registry/CONNECTORS";
export type { ConnectorRegistryEntry } from "./registry/CONNECTORS";
export { resolveConnector } from "./registry/resolveConnector";
