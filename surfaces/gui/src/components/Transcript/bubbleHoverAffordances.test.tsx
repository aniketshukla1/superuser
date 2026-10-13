import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Transcript } from "../Transcript";
import type { Item } from "../../types";
import { exactTime, timeAgo } from "../../when";

afterEach(cleanup);
describe("bubble hover affordances (FB-005)", () => {
  const TS = 1752969720; // unix seconds, as the server stamps them
  const ITEMS: Item[] = [
    { kind: "user", text: "post the digest", ts: TS },
    { kind: "assistant", text: "Done — posted to #all-hands." }, // pre-stamp history: no ts
  ];

  it("copy button copies the bubble's raw text and flashes Copied", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    render(<Transcript items={ITEMS} onApprove={vi.fn()} />);

    const copies = screen.getAllByTestId("bubble-copy");
    expect(copies).toHaveLength(2); // user + assistant bubbles both get one
    fireEvent.click(copies[0]);
    expect(writeText).toHaveBeenCalledWith("post the digest");
    // "Copied" lands only after the clipboard write RESOLVES (a rejected write must
    // not claim success), hence the await.
    await waitFor(() => expect(copies[0].textContent).toBe("Copied"));
    fireEvent.click(copies[1]);
    expect(writeText).toHaveBeenCalledWith("Done — posted to #all-hands.");
  });

  // The stamp reads as an AGE now, not a clock time: on a message you are looking at, "how
  // long ago" is the question. The wall-clock time stays in the title for when it is not.
  it("timestamp renders only when the item carries ts; full date rides the title", () => {
    render(<Transcript items={ITEMS} onApprove={vi.fn()} />);

    const stamps = screen.getAllByTestId("bubble-ts");
    expect(stamps).toHaveLength(1); // the ts-less assistant bubble shows none
    // The shared formatter's vocabulary: an age for a recent message, a date past a week.
    expect(stamps[0].textContent).toBe(timeAgo(TS));
    expect(stamps[0].getAttribute("title")).toBe(exactTime(TS));
  });

  // Every row stays on hover, including the last one (owner ask). A column of controls down
  // a transcript is noise whichever message it is on.
  it("keeps every meta row on hover, the last one included", () => {
    const { container } = render(
      <Transcript
        items={[
          { kind: "user", text: "q1" },
          { kind: "assistant", text: "a1", ts: TS },
          { kind: "user", text: "q2" },
          { kind: "assistant", text: "a2", ts: TS },
        ]}
        onApprove={vi.fn()}
      />,
    );
    const metas = [...container.querySelectorAll(".bubble-assistant [data-testid='bubble-copy']")].map(
      (b) => b.parentElement!.className,
    );
    expect(metas).toHaveLength(2);
    for (const cls of metas) expect(cls).toContain("opacity-0");
  });

  // "Try that again" means the most recent answer. Beside an older one it would silently
  // regenerate something else.
  it("offers retry on the last reply only, and never mid-turn", () => {
    const items: Item[] = [
      { kind: "user", text: "q1" },
      { kind: "assistant", text: "a1", ts: TS },
      { kind: "user", text: "q2" },
      { kind: "assistant", text: "a2", ts: TS },
    ];
    const onRetry = vi.fn();
    const { container, rerender } = render(
      <Transcript items={items} onRetry={onRetry} onApprove={vi.fn()} />,
    );
    const buttons = container.querySelectorAll("[data-testid='bubble-retry']");
    expect(buttons).toHaveLength(1);
    fireEvent.click(buttons[0]);
    expect(onRetry).toHaveBeenCalled();

    // A turn in flight has nothing finished to retry.
    rerender(<Transcript items={items} running onRetry={onRetry} onApprove={vi.fn()} />);
    expect(container.querySelectorAll("[data-testid='bubble-retry']")).toHaveLength(0);
  });

  // Reading aloud is a Tauri command. In a browser the button would throw, so it is not
  // offered at all — these tests run in jsdom, which is not the desktop shell.
  it("does not offer read-aloud outside the desktop shell", () => {
    const { container } = render(
      <Transcript
        items={[
          { kind: "user", text: "q" },
          { kind: "assistant", text: "a", ts: TS },
        ]}
        onApprove={vi.fn()}
      />,
    );
    expect(container.querySelectorAll("[data-testid='bubble-speak']")).toHaveLength(0);
  });
});

describe("failures in the transcript", () => {
  // A memory question that could not reach a model came back as an ordinary answer, with
  // the provider's JSON inside it and thumbs under it.
  it("renders a failed memory answer as a failure, with no feedback row", () => {
    const onOpenSettings = vi.fn();
    const { container } = render(
      <Transcript
        items={[
          { kind: "user", text: "@brain which repos have I starred?" },
          {
            kind: "assistant",
            text: "I couldn't search my memory just now — Add an API key for OpenAI in Settings → AI Models.",
            ts: 1752969720,
          },
        ]}
        onApprove={vi.fn()}
        onOpenSettings={onOpenSettings}
      />,
    );
    expect(screen.getByTestId("error-callout").textContent).toContain("There's no API key for OpenAI yet.");
    expect(container.querySelector(".bubble-assistant")).toBeNull();
    // Only the question keeps its copy row; the failure has none of an answer's controls.
    expect(container.querySelectorAll("[data-testid='bubble-copy']")).toHaveLength(1);
    expect(container.querySelector("[data-testid='feedback']")).toBeNull();
    fireEvent.click(screen.getByTestId("error-fix"));
    expect(onOpenSettings).toHaveBeenCalledWith("models");
  });

  it("keeps the raw reason one click away", () => {
    render(
      <Transcript
        items={[{ kind: "notice", tone: "warn", text: 'Error: model unreachable {"code": 7}', retriable: true }]}
        onApprove={vi.fn()}
      />,
    );
    expect(screen.queryByText(/"code": 7/)).toBeNull();
    fireEvent.click(screen.getByTestId("error-details"));
    expect(screen.getByText(/"code": 7/)).toBeTruthy();
  });

  it("offers Retry only on the tail failure of an idle session", () => {
    const failed: Item[] = [
      { kind: "user", text: "q" },
      { kind: "notice", tone: "warn", text: "Error: model down", retriable: true },
    ];
    const { rerender } = render(<Transcript items={failed} onApprove={vi.fn()} onRetry={vi.fn()} />);
    expect(screen.getByTestId("notice-retry")).toBeTruthy();
    rerender(<Transcript items={[...failed, { kind: "user", text: "again" }]} onApprove={vi.fn()} onRetry={vi.fn()} />);
    expect(screen.queryByTestId("notice-retry")).toBeNull();
  });

  it("leaves interruptions as the quiet line they were", () => {
    const { container } = render(
      <Transcript items={[{ kind: "notice", tone: "warn", text: "Interrupted." }]} onApprove={vi.fn()} />,
    );
    expect(container.querySelector(".notice.warn")?.textContent).toBe("Interrupted.");
    expect(screen.queryByTestId("error-callout")).toBeNull();
  });
});
