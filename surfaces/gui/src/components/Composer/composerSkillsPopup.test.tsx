import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Composer } from "../Composer";

const MENU = {
  skills: [
    { name: "weekly-report", description: "Monday status report", scope: "global", enabled: true },
    { name: "greet", description: "says hello", scope: "project", enabled: true },
    { name: "muted-one", description: "muted here", scope: "global", enabled: false },
  ],
};

function stubFetch() {
  const calls: { url: string; method: string }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init?: RequestInit) => {
      calls.push({ url, method: (init?.method || "GET").toUpperCase() });
      if (url.includes("/skills")) return { ok: true, json: async () => MENU } as Response;
      return { ok: true, json: async () => ({}) } as Response;
    }),
  );
  return calls;
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

const box = () => screen.getByPlaceholderText(/Type . for commands/);

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
describe("Composer / skills popup", () => {
  it("opens on a leading '/' and lists only enabled skills from the effective menu", async () => {
    stubFetch();
    render(<Composer {...props()} />);
    fireEvent.change(box(), { target: { value: "/" } });
    await screen.findByTestId("skill-popup");
    // The name leads; what it does follows it on the same line, and the whole thing
    // stays in the tooltip.
    expect(await screen.findByText("weekly-report")).toBeTruthy();
    expect(screen.getByText("greet")).toBeTruthy();
    expect(screen.queryByText("muted-one")).toBeNull(); // muted → not offered
    const greet = screen.getByRole("option", { name: /^greet/ });
    expect(greet.textContent).toContain("says hello");
    expect(greet.getAttribute("title")).toBe("says hello · project");
  });

  it("filters as you type", async () => {
    stubFetch();
    render(<Composer {...props()} />);
    fireEvent.change(box(), { target: { value: "/" } });
    await screen.findByText("weekly-report");
    fireEvent.change(box(), { target: { value: "/wee" } });
    expect(screen.getByText("weekly-report")).toBeTruthy();
    expect(screen.queryByText("greet")).toBeNull();
  });

  it("does NOT open for a mid-text slash", async () => {
    stubFetch();
    render(<Composer {...props()} />);
    fireEvent.change(box(), { target: { value: "rate 5/10 please" } });
    expect(screen.queryByTestId("skill-popup")).toBeNull();
  });

  it("selecting inserts /name inline; the send strips the prefix and carries the skill field", async () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    fireEvent.change(box(), { target: { value: "/gr" } });
    fireEvent.click(await screen.findByRole("option", { name: /^greet/ }));
    expect((box() as HTMLTextAreaElement).value).toBe("/greet "); // inline, no chip
    fireEvent.change(box(), { target: { value: "/greet say hi to the team" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(p.onSend).toHaveBeenCalled());
    expect(p.onSend).toHaveBeenCalledWith("say hi to the team", [], "greet");
  });

  it("a skill-only send works and Enter inside the popup never sends the query text", async () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    fireEvent.change(box(), { target: { value: "/wee" } });
    await screen.findByText("weekly-report");
    fireEvent.keyDown(box(), { key: "Enter" }); // selects, does not send
    expect(p.onSend).not.toHaveBeenCalled();
    expect((box() as HTMLTextAreaElement).value).toBe("/weekly-report ");
    fireEvent.keyDown(box(), { key: "Enter" }); // now sends, skill-only
    await waitFor(() => expect(p.onSend).toHaveBeenCalledWith("", [], "weekly-report"));
  });

  it("editing the /name prefix away un-picks the skill", async () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    fireEvent.change(box(), { target: { value: "/gr" } });
    fireEvent.click(await screen.findByRole("option", { name: /^greet/ }));
    fireEvent.change(box(), { target: { value: "hello plain" } }); // prefix gone
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(p.onSend).toHaveBeenCalledWith("hello plain", [], undefined));
  });

  it("Escape closes the popup and no popup ever opens without a sessionId", async () => {
    stubFetch();
    render(<Composer {...props()} />);
    fireEvent.change(box(), { target: { value: "/gr" } });
    await screen.findByTestId("skill-popup");
    fireEvent.keyDown(box(), { key: "Escape" });
    expect(screen.queryByTestId("skill-popup")).toBeNull();
    cleanup();
    stubFetch();
    render(<Composer {...props({ sessionId: undefined })} />);
    fireEvent.change(box(), { target: { value: "/" } });
    expect(screen.queryByTestId("skill-popup")).toBeNull();
  });
});

describe("Composer / built-in context commands", () => {
  it("lists commands under the skills and sends one on click", async () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    fireEvent.change(box(), { target: { value: "/" } });
    await screen.findByText("weekly-report");
    expect(screen.getByText("context")).toBeTruthy();
    expect(screen.getByText("compact")).toBeTruthy();
    // Skills stay first, so "/" + Enter picks a skill exactly as it did before,
    // with the built-in commands listed after them.
    const options = screen.getAllByRole("option");
    expect(options[0].textContent).toContain("weekly-report");
    expect(options.slice(-3).map((o) => o.textContent).join(" ")).toContain("compact");

    fireEvent.click(screen.getByRole("option", { name: /^context/ }));
    await waitFor(() => expect(p.onSend).toHaveBeenCalledWith("/context", [], undefined));
    expect((box() as HTMLTextAreaElement).value).toBe("");
  });

  it("a fully typed command sends on Enter instead of being swallowed by the popup", async () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    fireEvent.change(box(), { target: { value: "/compact" } });
    await screen.findByTestId("skill-popup");
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(p.onSend).toHaveBeenCalledWith("/compact", [], undefined));
  });

  it("filters commands out once the query stops matching, leaving skills alone", async () => {
    stubFetch();
    render(<Composer {...props()} />);
    fireEvent.change(box(), { target: { value: "/wee" } });
    await screen.findByText("weekly-report");
    expect(screen.queryByText("/context")).toBeNull();
    expect(screen.queryByText("/compact")).toBeNull();
  });

  it("a command is never sent as a skill", async () => {
    stubFetch();
    const p = props();
    render(<Composer {...p} />);
    fireEvent.change(box(), { target: { value: "/context" } });
    await screen.findByTestId("skill-popup");
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(p.onSend).toHaveBeenCalled());
    // Third arg is the skill field — a command must leave it undefined, and the
    // text must keep its slash so the backend recognizes the command.
    expect(p.onSend).toHaveBeenCalledWith("/context", [], undefined);
  });
});
