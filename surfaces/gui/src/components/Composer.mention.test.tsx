// The composer's "@" target popup. The menu is discoverability only — the message
// still goes out as plain text and the backend parses the mention out of it
// (`core::brain::parse_mention`) — so what these assert is that the popup opens for
// exactly the messages the backend will actually route, and never swallows a send.
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Composer } from "./Composer";

function stubFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      if (url.includes("/skills")) return { ok: true, json: async () => ({ skills: [] }) } as Response;
      return { ok: true, json: async () => ({}) } as Response;
    }),
  );
}

const props = (extra: Partial<Parameters<typeof Composer>[0]> = {}) => ({
  mode: "interactive",
  model: "gpt-5.6-sol",
  running: false,
  connected: true,
  sessionId: "s1",
  onSend: vi.fn(),
  onInterrupt: vi.fn(),
  onModeChange: vi.fn(),
  onModelChange: vi.fn(),
  ...extra,
});

const box = () => screen.getByPlaceholderText(/Type . for commands/) as HTMLTextAreaElement;
const type = (value: string) => fireEvent.change(box(), { target: { value } });
const enter = () => fireEvent.keyDown(box(), { key: "Enter" });

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("Composer / @ mentions", () => {
  it("opens on a leading '@' and offers the brain", () => {
    stubFetch();
    render(<Composer {...props()} />);
    type("@");
    expect(screen.getByTestId("mention-popup")).toBeTruthy();
    const row = screen.getByRole("option", { name: /^@brain/ });
    expect(row.getAttribute("title")).toMatch(/memories, timeline, knowledge and profile/);
  });

  it("filters while typing and completes the name on Enter", () => {
    stubFetch();
    render(<Composer {...props()} />);
    type("@br");
    expect(screen.getByTestId("mention-popup")).toBeTruthy();
    enter();
    // Completed inline with the trailing space, ready for the question. Nothing sent:
    // "@brain" on its own is a half-written message.
    expect(box().value).toBe("@brain ");
    expect(screen.queryByTestId("mention-popup")).toBeNull();
  });

  it("clicking a row completes it too", () => {
    stubFetch();
    render(<Composer {...props()} />);
    type("@");
    fireEvent.click(screen.getByRole("option", { name: /^@brain/ }));
    expect(box().value).toBe("@brain ");
  });

  // The backend only routes a mention that OPENS the message, so a menu that appeared
  // mid-sentence would be offering something that does not happen.
  it("stays shut when the '@' is not the first character", () => {
    stubFetch();
    render(<Composer {...props()} />);
    type("ask @br");
    expect(screen.queryByTestId("mention-popup")).toBeNull();
  });

  it("stays shut when nothing matches, and does not swallow the send", () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    type("@someone@example.com");
    expect(screen.queryByTestId("mention-popup")).toBeNull();
    enter();
    expect(p.onSend).toHaveBeenCalledWith("@someone@example.com", [], undefined);
  });

  it("sends the question as ordinary text — the mention is not a wire field", () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    type("@brain what did I decide about Postgres?");
    // Whitespace after the target closes the menu, so Enter sends.
    expect(screen.queryByTestId("mention-popup")).toBeNull();
    enter();
    expect(p.onSend).toHaveBeenCalledWith(
      "@brain what did I decide about Postgres?",
      [],
      undefined,
    );
    expect(box().value).toBe("");
  });

  // The other assistants: `@expenses …` hands that message to Expense Manager. Matched on
  // the handle or the name; never the conversation's own, never one switched off.
  it("offers the other assistants by handle or name, and completes the handle", async () => {
    const people = [
      { id: "chief", name: "Chief of Staff", tagline: "Digest and priorities", enabled: true },
      { id: "expenses", name: "Expense Manager", tagline: "Reconcile weekly expenses", enabled: true },
      { id: "analyst", name: "Data Analyst", tagline: "", enabled: true },
      { id: "off", name: "Switched Off", tagline: "", enabled: false },
    ];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) =>
        ({ ok: true, json: async () => (url.includes("/v1/personas") ? { personas: people } : { skills: [] }) }) as Response,
      ),
    );
    render(<Composer {...props({ agent: "chief" })} />);
    type("@exp");
    const row = await screen.findByRole("option", { name: /^@expenses/ });
    expect(row.getAttribute("title")).toBe("Expense Manager — Reconcile weekly expenses");
    type("@data");
    expect(screen.getByRole("option", { name: /^@analyst/ })).toBeTruthy();
    type("@");
    expect(screen.getByRole("option", { name: /^@brain/ })).toBeTruthy();
    expect(screen.queryByRole("option", { name: /^@chief/ })).toBeNull();
    expect(screen.queryByRole("option", { name: /^@off/ })).toBeNull();
    fireEvent.click(screen.getByRole("option", { name: /^@expenses/ }));
    expect(box().value).toBe("@expenses ");
  });

  it("Escape clears the half-typed target", () => {
    stubFetch();
    render(<Composer {...props()} />);
    type("@bra");
    fireEvent.keyDown(box(), { key: "Escape" });
    expect(box().value).toBe("");
    expect(screen.queryByTestId("mention-popup")).toBeNull();
  });
});
