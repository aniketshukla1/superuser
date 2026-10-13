import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { Transcript } from "./Transcript";
import { itemsFromMessages } from "../itemsFromMessages";

afterEach(cleanup);

// A turn that used a tool must still show what the assistant SAID.
//
// The turn group collapses a turn's activity behind "N steps", and the final answer
// is meant to render as a bubble outside it — `Transcript` finds that answer by
// popping trailing assistant items. A reopened conversation replayed its stored
// assistant message as text-then-tools, so the answer never trailed and went inside
// the disclosure. On screen: "› 1 step", a chart, and no reply.
//
// Live streaming was unaffected, so this only appeared after switching chats and
// coming back — the text was on disk the whole time.

const CHART = {
  spec: { type: "bar", title: "AI Trends", x: { field: "a" }, y: { field: "b" }, data: [{ a: "x", b: 1 }] },
  source: "McKinsey 2026",
};

describe("a reopened turn that drew a chart", () => {
  const stored = [
    { role: "user", content: "chart the AI trends" },
    {
      role: "assistant",
      content: "Enterprise adoption is the standout figure.",
      items: [{ kind: "tool", name: "chart", args: CHART, status: "ok" }],
    },
  ] as any;

  it("shows the answer as a bubble, not folded into the step group", () => {
    render(<Transcript items={itemsFromMessages(stored)} onApprove={vi.fn()} />);
    const answer = screen.getByText("Enterprise adoption is the standout figure.");
    expect(answer).toBeTruthy();
    // The whole point: not inside the collapsed disclosure.
    expect(answer.closest("[data-testid=turn-group]")).toBeNull();
    expect(answer.closest("details")).toBeNull();
  });

  it("still draws the chart, and still collapses the step", () => {
    render(<Transcript items={itemsFromMessages(stored)} onApprove={vi.fn()} />);
    expect(screen.getByTestId("transcript-chart")).toBeTruthy();
    expect(screen.getByText(/1 step/)).toBeTruthy();
  });
});
