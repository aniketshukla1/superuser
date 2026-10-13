import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { InboxItemCard } from "./InboxItemCard";
import type { InboxItem } from "../api";

afterEach(cleanup);

const parked = (data: Record<string, any>, body: string): InboxItem => ({
  id: "ask-1",
  session_id: "email-abc",
  kind: "approval",
  title: "Run `send_message`?",
  body,
  state: "pending",
  resolution: null,
  inbox: "default",
  created_at: "",
  resolved_at: null,
  data,
});

describe("InboxItemCard — a parked message", () => {
  // The email bot parks every reply it drafts. Approving one means reading it, and the
  // summary line stops at 140 characters — the card has to offer the whole reply. It opens
  // clamped, like the live approval card, with the rest one click away.
  it("offers all of its text, not just the summary line", () => {
    const text = `Re: Deck for Monday\n\n${"Here is the deck, with the numbers from Friday. ".repeat(6)}\n\nBest,\nAniket`;
    const summary = `Reply to ada@example.com — Re: Deck for Monday: ${text.slice(21, 161)}`;
    const { container } = render(
      <InboxItemCard
        item={parked({ tool: "send_message", arguments: { target: "email:ada@example.com", text } }, summary)}
        onResolve={vi.fn()}
      />,
    );
    expect(container.textContent).toContain("Re: Deck for Monday");
    fireEvent.click(screen.getByText(/show all 6 lines/));
    expect(container.textContent).toContain("Best,\nAniket");
  });
});
