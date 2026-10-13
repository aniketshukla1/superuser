// First-run onboarding (UX-DECISIONS §24 → §29 → §39): model → your tools → go.
// §39 (owner design, 2026-07-18): step 1 is a PROVIDER GALLERY — 13 real brand
// marks, two per row, each card wearing its own state — and step 2 is a
// two-state tools page whose post-sign-in body is a mini connector gallery with
// live one-click connects. Both steps share one frame rule: the header and
// footer never move; only the middle region swaps, at a fixed height.
// The gallery/form themselves live in providers/ProviderSetup.tsx, shared with
// Settings ▸ Models (UX-021) so the two surfaces can't drift.
// Replayable from Settings ▸ General ▸ "Run setup again".

// Step 2's benefit rows (§41): managed connectors with LIVE prod OAuth apps only,
// each framed by the job it does (detail copy stays ONE line even with a Connect
// pill — wrap made rows jump between states). gmail + google_calendar ship as one
// combined grayed "Coming soon" row — both ride the same Google app, gated on
// Google verification/CASA; give them rows when it lands.
export const TOOL_ROWS = [
  { name: "outlook", benefit: "Stay on top of email", detail: "Outlook — triage mail, draft replies, run your calendar." },
  { name: "slack", benefit: "Keep up with Slack", detail: "Slack — catch up, answer mentions, post updates." },
  { name: "github", benefit: "Ship code", detail: "GitHub — review PRs, watch issues, reply to @mentions." },
  { name: "notion", benefit: "Keep your notes in reach", detail: "Notion — search pages, query databases, draft docs." },
  { name: "hubspot", benefit: "Keep the CRM current", detail: "HubSpot — update deals, log notes, prep calls." },
  { name: "attio", benefit: "Track every relationship", detail: "Attio — search records, read timelines, log notes." },
];
export const TOOLS_SOON = ["gmail", "google_calendar"];
