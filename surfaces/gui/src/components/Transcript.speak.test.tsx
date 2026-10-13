import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

// Hoisted above the Transcript import on purpose: `canSpeak` is computed once at module
// load (isTauri() cannot change while the app runs), so the desktop and browser cases
// cannot both be exercised in one file. This file is the desktop one; the browser case
// lives in Transcript.test.tsx, which runs in plain jsdom.
const speak = vi.fn(() => Promise.resolve());
const stopSpeaking = vi.fn(() => Promise.resolve());
vi.mock("../tauri", () => ({
  isTauri: () => true,
  speak: (...a: unknown[]) => speak(...(a as [])),
  stopSpeaking: () => stopSpeaking(),
}));

const { Transcript } = await import("./Transcript");
import type { Item } from "../types";

afterEach(() => {
  cleanup();
  speak.mockClear();
  stopSpeaking.mockClear();
});

const ITEMS: Item[] = [
  { kind: "user", text: "q" },
  { kind: "assistant", text: "the answer text", ts: 1_780_000_000 },
];

describe("reading a reply aloud", () => {
  it("speaks the reply's text, and the same control stops it", async () => {
    render(<Transcript items={ITEMS} onApprove={vi.fn()} />);
    const btn = screen.getByTestId("bubble-speak");

    // The message, not a label or a truncation of it.
    expect(btn.getAttribute("aria-label")).toBe("Read aloud");
    fireEvent.click(btn);
    expect(speak).toHaveBeenCalledWith("the answer text");

    // It becomes Stop and STAYS Stop. `speak` resolves when the shell has queued the text,
    // not when it has finished saying it, so resetting on resolve would flip the control
    // back while the machine was still talking and leave no way to stop it.
    expect(await screen.findByLabelText("Stop reading")).toBeTruthy();
    fireEvent.click(screen.getByTestId("bubble-speak"));
    expect(stopSpeaking).toHaveBeenCalled();
    expect(screen.getByLabelText("Read aloud")).toBeTruthy();
  });

  it("reverts the control when the shell refuses to speak at all", async () => {
    speak.mockImplementationOnce(() => Promise.reject(new Error("no voice")));
    render(<Transcript items={ITEMS} onApprove={vi.fn()} />);
    fireEvent.click(screen.getByTestId("bubble-speak"));
    // A button stuck reading "Stop" for speech that never started cannot be pressed back.
    expect(await screen.findByLabelText("Read aloud")).toBeTruthy();
  });
});
