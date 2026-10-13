// AccessSection — "what can this session touch". It owns the workspace panel's Access TAB.
//
// It began as a collapsible section stacked in the old right rail, where a header and a
// one-line summary were how you knew what was inside without opening it. Access has its
// own tab now, so that header was a second door in front of a door: the tab is already
// the disclosure, and the summary described content the tab shows in full. Both are gone
// and the body renders flat.
//   · Sources — Connected toggles (per-session mute), Recommended (connect-in-context), and the
//     two-way connectors' channels drill-down — the drawer's content, recut.
//   · Folders — the session's working directories (add/remove, RO/RW gate, branch).
// Owns its data (GET /v1/sessions/{id}/connections + the connector index), like the settings
// row before it. Deep links (intro "Configure ›", onboarding "Start working") bump `openKey`
// to expand it and scroll it into view.

export { ScopePicker } from "./AccessSection/ScopePicker";
export { AccessSection } from "./AccessSection/AccessSection";
