// PersonaView — the persona detail page (§5, mock parity). Identity header + Enable toggle, About,
// Built-in capabilities (tools), "Connections for full benefit" (manifest `recommends`, core/optional
// + reason + connect state), "New sessions get by default" (persona-default connection toggles), and a
// defaults footer (recommended models / default mode / workspace).
//
// Data: fetches GET /v1/personas/{id} on mount; also fetches /v1/connectors to thread real brand
// colors (Phase 1's `brand_color`) into the badges via visualFor(). Toggling a default connection
// POSTs /v1/personas/{id}/connections and applies the returned `default_connections` (re-read).
// Enabling/disabling POSTs /v1/personas/{id}/enable.

export { PersonaView } from "./PersonaView/PersonaView";
