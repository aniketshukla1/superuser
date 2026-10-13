// Which parked items a conversation should still show a card for.
//
// The server now filters `GET /v1/inbox` by `session_id` and `state`, which it had
// never done despite the client sending both since it was written. The visible
// consequence was a question parked by an unattended automation run hours earlier,
// already denied, rendering as a live card in someone's chat — and clicking an
// option did nothing, because there was nothing left to resolve.
//
// This is the client's half of that, and it is deliberately narrower than the
// server's. Only `state` is re-checked here:
//
//   * **Resolved is unambiguous.** An item with an answer cannot be answered again,
//     so a card for one can only ever mislead. Re-checking costs nothing and closes
//     the window between a stale fetch and the next poll.
//   * **Session scoping is not.** An item can be surfaced to a conversation other
//     than the one that raised it, and the server is the only place that knows the
//     routing rules. Filtering on `session_id` here would hide cards the server
//     meant to show — trading a visible bug for an invisible one.

import type { InboxItem } from "./api";

/** The items still worth a card. */
export function stillOpen(items: InboxItem[]): InboxItem[] {
  // `state` is absent on payloads from a server older than the field. Absent means
  // unknown, and hiding a card because a field is missing is the worse failure of
  // the two — the same rule this project applies to an unreported capability.
  return items.filter((i) => i.state !== "resolved");
}

/** Whether this item can still be acted on. */
export function isOpen(item: InboxItem | undefined): boolean {
  return !!item && item.state !== "resolved";
}
