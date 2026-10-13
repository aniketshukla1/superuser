// Steering: a message typed while the agent is working is a correction, not an
// error. Before this the composer refused to send at all mid-turn, so the only
// thing you could do to a run going the wrong way was kill it.
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Composer } from "./Composer";

const props = (extra: Partial<Parameters<typeof Composer>[0]> = {}) => ({
  mode: "interactive",
  model: "gpt-5.6-sol",
  running: false,
  connected: true,
  onSend: vi.fn(),
  onInterrupt: vi.fn(),
  onModeChange: vi.fn(),
  onModelChange: vi.fn(),
  ...extra,
});

const type = (value: string) =>
  fireEvent.change(screen.getByRole("textbox"), { target: { value } });

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("Composer while a turn is running", () => {
  it("sends what you type instead of swallowing it", () => {
    const p = props({ running: true });
    render(<Composer {...p} />);

    type("actually, use the staging database");
    fireEvent.click(screen.getByTestId("composer-steer"));

    expect(p.onSend).toHaveBeenCalledWith("actually, use the staging database", [], undefined);
  });

  it("keeps Stop available beside Send — redirecting and halting are different", () => {
    const p = props({ running: true });
    render(<Composer {...p} />);
    type("no, the other file");

    expect(screen.getByTestId("composer-stop")).toBeTruthy();
    expect(screen.getByTestId("composer-steer")).toBeTruthy();

    fireEvent.click(screen.getByTestId("composer-stop"));
    expect(p.onInterrupt).toHaveBeenCalled();
    expect(p.onSend).not.toHaveBeenCalled();
  });

  it("offers Send mid-turn only when there is something to send", () => {
    render(<Composer {...props({ running: true })} />);
    expect(screen.queryByTestId("composer-steer")).toBeNull();
    expect(screen.getByTestId("composer-stop")).toBeTruthy();
  });

  it("says the composer is live rather than looking disabled", () => {
    render(<Composer {...props({ running: true })} />);
    expect(screen.getByPlaceholderText("Working… type to steer it")).toBeTruthy();
  });

  it("still sends normally when nothing is running", () => {
    const p = props({ running: false });
    render(<Composer {...p} />);
    type("hello");
    fireEvent.click(screen.getByLabelText("Send"));
    expect(p.onSend).toHaveBeenCalledWith("hello", [], undefined);
  });
});
