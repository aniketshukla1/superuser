import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import { createRef } from "react";
import { TranscriptNav } from "./TranscriptNav";
import type { Item } from "../types";

const user = (text: string): Item => ({ kind: "user", text });
const bot = (text: string): Item => ({ kind: "assistant", text });

// The component reads geometry to decide which turn you are on. jsdom has no layout, so a
// scroller stub keeps that path from throwing; the JUMP itself is browser behaviour and is
// verified against a real one (Chrome, 6-message session: clicking the first question moved
// .main-scroll from 364 to 63, and "Session start" to 0).
const scroller = () => {
  const el = document.createElement("div");
  el.addEventListener = vi.fn() as any;
  el.removeEventListener = vi.fn() as any;
  const ref = createRef<HTMLElement>();
  (ref as { current: HTMLElement }).current = el;
  return ref;
};

// `globals` is off in vitest.config.ts, so Testing Library does not auto-cleanup — without
// this each test inherits the previous one's DOM and getByTestId finds two navs.
afterEach(cleanup);

describe("TranscriptNav", () => {
  it("lists what you asked, newest last, with a way back to the top", () => {
    render(
      <TranscriptNav
        scrollRef={scroller()}
        items={[user("What does # mean?"), bot("Root."), user("And $?"), bot("A user.")]}
      />,
    );
    const nav = screen.getByTestId("transcript-nav");
    const labels = within(nav)
      .getAllByRole("button")
      .map((b) => b.textContent);
    expect(labels).toEqual(["Session start", "What does # mean?", "And $?"]);
  });

  // The replies are answers to these questions, so listing them too would be the same
  // conversation written out twice.
  it("lists questions only, not replies or tool calls", () => {
    render(
      <TranscriptNav
        scrollRef={scroller()}
        items={[
          user("first"),
          { kind: "tool", id: "t1", name: "bash", args: {}, status: "ok" },
          bot("a long answer"),
          user("second"),
        ]}
      />,
    );
    const labels = within(screen.getByTestId("transcript-nav"))
      .getAllByRole("button")
      .map((b) => b.textContent);
    expect(labels).toEqual(["Session start", "first", "second"]);
  });

  // A map of one thing is not a map — it would be chrome on every short conversation,
  // which is most of them.
  it("stays out of the way until there is something to navigate", () => {
    const { container, rerender } = render(
      <TranscriptNav scrollRef={scroller()} items={[user("only one"), bot("reply")]} />,
    );
    expect(container.querySelector('[data-testid="transcript-nav"]')).toBeNull();

    rerender(
      <TranscriptNav
        scrollRef={scroller()}
        items={[user("one"), bot("reply"), user("two")]}
      />,
    );
    expect(screen.getByTestId("transcript-nav")).toBeTruthy();
  });

  // A multi-line paste should give a readable one-line entry, not a wall.
  it("takes the first non-empty line as the label", () => {
    render(
      <TranscriptNav
        scrollRef={scroller()}
        items={[user("\n\n  Fix   the   build \nand then deploy"), user("second")]}
      />,
    );
    expect(screen.getByTitle("Fix the build")).toBeTruthy();
  });
});
