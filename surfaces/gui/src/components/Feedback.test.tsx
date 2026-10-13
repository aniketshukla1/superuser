import { describe, expect, it, vi, beforeEach } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Feedback } from "./Feedback";

const sendFeedback = vi.fn().mockResolvedValue(undefined);
vi.mock("../api", () => ({
  sendFeedback: (...args: unknown[]) => sendFeedback(...args),
}));

const reasons = [
  { id: "made_it_up", label: "It made something up" },
  { id: "wrong_source", label: "It used the wrong file or memory" },
];

describe("Feedback", () => {
  beforeEach(() => {
    cleanup();
    sendFeedback.mockClear();
  });

  it("files an upvote without asking anything", async () => {
    // "This was good" needs no taxonomy — asking for one would tax the answer the
    // user liked, which is the wrong way round.
    render(<Feedback sessionId="s1" messageAt={1234.5} reasons={reasons} />);
    fireEvent.click(screen.getByLabelText("This was good"));
    await waitFor(() => expect(sendFeedback).toHaveBeenCalledOnce());
    expect(sendFeedback.mock.calls[0][0]).toMatchObject({ session: "s1", messageAt: 1234.5, verdict: "up" });
    expect(sendFeedback.mock.calls[0][0].reason).toBeUndefined();
  });

  it("a downvote asks what was wrong before it sends anything", async () => {
    // The load-bearing rule, and the same one `ask_user` is held to: a verdict with
    // no reason is a number nobody can act on, so the click opens the options rather
    // than filing "bad".
    render(<Feedback sessionId="s1" messageAt={1} reasons={reasons} />);
    fireEvent.click(screen.getByLabelText("Something was wrong"));
    expect(sendFeedback).not.toHaveBeenCalled();
    expect(screen.getByText("What was wrong?")).toBeTruthy();

    fireEvent.click(screen.getByText("It made something up"));
    await waitFor(() => expect(sendFeedback).toHaveBeenCalledOnce());
    expect(sendFeedback.mock.calls[0][0]).toMatchObject({ verdict: "down", reason: "made_it_up" });
  });

  it("carries an optional note along with the reason", async () => {
    render(<Feedback sessionId="s1" messageAt={1} reasons={reasons} />);
    fireEvent.click(screen.getByLabelText("Something was wrong"));
    fireEvent.change(screen.getByLabelText("Optional note"), { target: { value: "invented the file name" } });
    fireEvent.click(screen.getByText("It used the wrong file or memory"));
    await waitFor(() => expect(sendFeedback).toHaveBeenCalledOnce());
    expect(sendFeedback.mock.calls[0][0].note).toBe("invented the file name");
  });

  it("cancelling files nothing", async () => {
    render(<Feedback sessionId="s1" messageAt={1} reasons={reasons} />);
    fireEvent.click(screen.getByLabelText("Something was wrong"));
    fireEvent.click(screen.getByText("Cancel"));
    expect(sendFeedback).not.toHaveBeenCalled();
    expect(screen.getByLabelText("This was good")).toBeTruthy();
  });

  it("renders nothing without a session and a message to attach to", () => {
    // A verdict filed against the wrong turn is worse than one not filed at all.
    const { container } = render(<Feedback messageAt={1} reasons={reasons} />);
    expect(container.innerHTML).toBe("");
    const b = render(<Feedback sessionId="s1" reasons={reasons} />);
    expect(b.container.innerHTML).toBe("");
  });

  it("shows the verdict already given", () => {
    render(<Feedback sessionId="s1" messageAt={1} reasons={reasons} given="up" />);
    expect(screen.getByLabelText("This was good").getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByLabelText("Something was wrong").getAttribute("aria-pressed")).toBe("false");
  });
});
