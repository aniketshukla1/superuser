import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { LiveView } from "./LiveView";

// The live view's promises: nothing the user does is sent unless they hold the wheel;
// what is sent lands where they pointed, in the page's own pixels; and closing while
// holding the wheel asks what to do with it rather than deciding silently.

class FakeSocket {
  static OPEN = 1;
  static last: FakeSocket | null = null;
  readyState = 1;
  sent: any[] = [];
  onmessage: ((e: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  constructor(public url: string) {
    FakeSocket.last = this;
  }
  send(data: string) {
    this.sent.push(JSON.parse(data));
  }
  close() {}
  push(m: any) {
    act(() => this.onmessage?.({ data: JSON.stringify(m) }));
  }
}

let posts: any[] = [];

beforeEach(() => {
  posts = [];
  vi.stubGlobal("WebSocket", FakeSocket as any);
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      if (init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        posts.push({ url: String(url), body });
        return { ok: true, json: async () => ({ ok: true, control: { holder: body.action === "take" ? "user" : "assistant", since: 1, requested: false } }) } as Response;
      }
      return {
        ok: true,
        json: async () => ({
          persona: "assistant",
          running: true,
          url: "https://example.com/login",
          title: "Sign in",
          control: { holder: "assistant", since: 0, requested: false },
          has_profile: true,
          viewers: 1,
          last_used: 0,
        }),
      } as Response;
    }),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

const frame = { type: "frame", data: "AAAA", width: 1280, height: 800 };

function screenAt(el: HTMLElement) {
  el.getBoundingClientRect = () => ({ left: 100, top: 50, width: 640, height: 400, right: 740, bottom: 450, x: 100, y: 50, toJSON: () => ({}) });
  return el;
}

describe("the live view", () => {
  it("sends nothing while the assistant has the wheel", async () => {
    render(<LiveView persona="assistant" name="Assistant" onClose={vi.fn()} />);
    const ws = FakeSocket.last!;
    expect(ws.url).toContain("/ws/computer/assistant");
    ws.push(frame);
    ws.push({ type: "control", holder: "assistant", since: 0, requested: false });
    const el = screenAt(await screen.findByTestId("live-view-screen"));
    fireEvent.mouseDown(el, { clientX: 420, clientY: 250, button: 0 });
    fireEvent.keyDown(el, { key: "a", code: "KeyA" });
    expect(ws.sent).toEqual([]);
    expect(screen.getByTestId("live-view-holder").textContent).toBe("Assistant has control");
  });

  it("once the user holds it, a click lands in the page's own pixels", async () => {
    render(<LiveView persona="assistant" name="Assistant" onClose={vi.fn()} />);
    const ws = FakeSocket.last!;
    ws.push(frame);
    ws.push({ type: "control", holder: "user", since: 1, requested: false });
    const el = screenAt(await screen.findByTestId("live-view-screen"));
    // 320px into a 640px-wide picture of a 1280px page is x=640.
    fireEvent.mouseDown(el, { clientX: 420, clientY: 250, button: 0 });
    expect(ws.sent[0]).toMatchObject({ type: "mouse", event: "pressed", x: 640, y: 400, button: "left", clickCount: 1 });
    fireEvent.keyDown(el, { key: "a", code: "KeyA" });
    expect(ws.sent[1]).toMatchObject({ type: "key", event: "down", key: "a", text: "a" });
    // A shortcut is a key, not a character to type.
    fireEvent.keyDown(el, { key: "a", code: "KeyA", ctrlKey: true });
    expect(ws.sent[2].text).toBeUndefined();
    expect(ws.sent[2].modifiers).toBe(2);
  });

  it("take control asks the server, not the socket", async () => {
    render(<LiveView persona="assistant" name="Assistant" onClose={vi.fn()} />);
    FakeSocket.last!.push(frame);
    fireEvent.click(await screen.findByTestId("live-view-take"));
    await waitFor(() => expect(posts).toEqual([{ url: expect.stringContaining("/v1/computers/assistant/control"), body: { action: "take" } }]));
    await screen.findByTestId("live-view-release");
  });

  it("closing while holding the wheel asks what to do with it", async () => {
    const onClose = vi.fn();
    render(<LiveView persona="assistant" name="Assistant" onClose={onClose} />);
    const ws = FakeSocket.last!;
    ws.push(frame);
    ws.push({ type: "control", holder: "user", since: 1, requested: false });
    fireEvent.click(await screen.findByTestId("live-view-close"));
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("live-view-close-release"));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(posts.map((p) => p.body)).toEqual([{ action: "release" }]);
  });

  it("shows why the assistant is asking", async () => {
    render(<LiveView persona="assistant" name="Assistant" onClose={vi.fn()} />);
    FakeSocket.last!.push({ type: "control", holder: "assistant", since: 0, requested: true, reason: "Sign in to example.com" });
    expect((await screen.findByTestId("live-view-ask")).textContent).toContain("Sign in to example.com");
  });
});
