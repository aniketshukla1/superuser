// A prefill is text another surface hands the composer — "Create with Superuser" in
// Settings ▸ Skills starts a chat with "Build a new skill for me: …" waiting in the box.
// Coming from Settings, the composer MOUNTS with the prefill already set, and a dev build
// runs mount effects twice: the conversation-change clear used to run again after the
// prefill had landed, and the box opened empty.
import { StrictMode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
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

const box = () => screen.getByRole("textbox") as HTMLTextAreaElement;

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("Composer prefill", () => {
  it("keeps a prefill it mounts with, even when mount effects run twice", () => {
    render(
      <StrictMode>
        <Composer {...props({ resetKey: "s-new", prefill: { text: "Build a new skill for me: ", nonce: 1 } })} />
      </StrictMode>,
    );
    expect(box().value).toBe("Build a new skill for me: ");
  });

  it("still clears a draft when the conversation changes, then lands a prefill sent with it", () => {
    const { rerender } = render(
      <StrictMode>
        <Composer {...props({ resetKey: "s-1" })} />
      </StrictMode>,
    );
    rerender(
      <StrictMode>
        <Composer {...props({ resetKey: "s-1", prefill: { text: "half-typed", nonce: 1 } })} />
      </StrictMode>,
    );
    expect(box().value).toBe("half-typed");

    // Another conversation: the old draft must not follow you there…
    rerender(
      <StrictMode>
        <Composer {...props({ resetKey: "s-2", prefill: { text: "half-typed", nonce: 1 } })} />
      </StrictMode>,
    );
    expect(box().value).toBe("");

    // …and a new chat opened WITH a prefill gets the prefill, not the clear.
    rerender(
      <StrictMode>
        <Composer {...props({ resetKey: "s-3", prefill: { text: "Build a new skill for me: ", nonce: 2 } })} />
      </StrictMode>,
    );
    expect(box().value).toBe("Build a new skill for me: ");
  });
});
