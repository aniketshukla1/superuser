// From GET /v1/sessions/{id}/messages to the GUI's `Item[]`, kept out of App.tsx so it can be
// tested by itself. A user message that came in through a connector carries a `source`
// record (§3.1) and becomes a `connector` item (a ConnectorMessageCard) rather than a plain
// bubble. The connector registry decides how each one looks, so nothing here is Slack-specific.

export { settleRunningTools } from "./itemsFromMessages/settleRunningTools";
export { itemsFromMessages, mergeJournalSteps, userItemFromContent } from "./itemsFromMessages/itemsFromMessages";
