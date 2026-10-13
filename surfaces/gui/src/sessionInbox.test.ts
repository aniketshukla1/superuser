import { describe, it, expect } from "vitest";
import { stillOpen, isOpen } from "./sessionInbox";
import type { InboxItem } from "./api";

const item = (over: Partial<InboxItem> = {}): InboxItem =>
  ({
    id: "ask-1",
    session_id: "chat-1",
    kind: "question",
    title: "What would you like the brief to cover?",
    body: "",
    state: "pending",
    resolution: null,
    inbox: "main",
    created_at: "2026-09-03T02:16:32+00:00",
    resolved_at: null,
    ...over,
  }) as InboxItem;

describe("which parked items still deserve a card", () => {
  it("drops one that has already been answered", () => {
    // The reported bug, exactly: a question an unattended run parked hours earlier
    // and that was already denied, rendering as a live card. Clicking an option did
    // nothing, because there was nothing left to resolve.
    const stale = item({ state: "resolved", resolution: "denied", session_id: "__run__run-f56" });
    expect(stillOpen([stale, item()])).toEqual([item()]);
    expect(isOpen(stale)).toBe(false);
  });

  it("keeps one whose state the server never sent", () => {
    // Absent is not resolved. A payload from a server older than the field would
    // otherwise hide every card, which is the worse of the two failures — the same
    // rule this project applies to an unreported model capability.
    const old = item({ state: undefined as unknown as InboxItem["state"] });
    expect(stillOpen([old])).toHaveLength(1);
    expect(isOpen(old)).toBe(true);
  });

  it("does not second-guess which session an item belongs to", () => {
    // An item can be surfaced to a conversation other than the one that raised it,
    // and only the server knows the routing rules. Filtering on `session_id` here
    // would hide cards it meant to show — a visible bug traded for an invisible one.
    const elsewhere = item({ session_id: "__run__run-f56" });
    expect(stillOpen([elsewhere])).toHaveLength(1);
  });

  it("has nothing to show when nothing is open", () => {
    expect(stillOpen([])).toEqual([]);
    expect(isOpen(undefined)).toBe(false);
  });
});
