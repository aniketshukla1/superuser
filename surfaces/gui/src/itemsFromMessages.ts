// Maps the raw transcript from GET /v1/sessions/{id}/messages into the GUI's `Item[]` model.
// Extracted from App.tsx so it can be unit-tested without standing up the whole app.
//
// A connector-delivered user message carries a structured `source` sidecar (§3.1); when present it
// becomes a `connector` item (rendered as ConnectorMessageCard) instead of a plain user bubble. This
// generalizes to any connector via the registry — no Slack special-casing here.

export { itemsFromMessages, mergeJournalSteps, userItemFromContent } from "./itemsFromMessages/itemsFromMessages";
export { settleRunningTools } from "./itemsFromMessages/settleRunningTools";
