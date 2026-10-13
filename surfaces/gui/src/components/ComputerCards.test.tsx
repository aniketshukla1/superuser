import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ComputerHelpCard, ComputerTile, SecretCard } from "./ComputerCards";

// What these cards promise: a value typed into the masked box goes to the page and is gone
// from the screen the moment it lands; "Not now" is an answer the assistant hears; a card
// whose request the browser no longer has closes itself; and a kept screen that has been
// deleted says so instead of drawing something else.

type Call = { url: string; method?: string; body: any };

function stubFetch(routes: { status?: any; post?: (url: string, body: any) => any; frame?: any } = {}) {
  const calls: Call[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    const u = String(url);
    const body = init?.body ? JSON.parse(String(init.body)) : null;
    calls.push({ url: u, method: init?.method, body });
    const reply = (v: any) => ({ ok: true, json: async () => v }) as Response;
    if (init?.method === "POST") return reply(routes.post ? routes.post(u, body) : { ok: true });
    if (u.includes("/frames/")) return reply(routes.frame ?? {});
    if (u.includes("/v1/computers/")) return reply(routes.status ?? {});
    return reply({});
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

const status = (control: any) => ({
  persona: "assistant",
  running: true,
  url: "https://example.com/login",
  title: "Sign in",
  control: { holder: "assistant", since: 0, requested: false, ...control },
  has_profile: true,
  viewers: 0,
  last_used: 0,
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe("the masked box", () => {
  it("sends the value once, then shows only how many characters landed", async () => {
    const calls = stubFetch({
      status: status({ secret_wanted: "Password" }),
      post: () => ({ ok: true, chars: 7 }),
    });
    const onDone = vi.fn();
    render(<SecretCard persona="assistant" name="Assistant" label="Password" onDone={onDone} />);
    const input = screen.getByTestId("secret-card-input") as HTMLInputElement;
    expect(input.type).toBe("password");
    fireEvent.change(input, { target: { value: "hunter2" } });
    fireEvent.click(screen.getByTestId("secret-card-send"));
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("sent"));
    const sent = calls.filter((c) => c.method === "POST");
    expect(sent).toEqual([{ url: expect.stringContaining("/v1/computers/assistant/secret"), method: "POST", body: { value: "hunter2" } }]);
    expect(screen.getByTestId("secret-card-sent").textContent).toContain("7 characters");
    expect(document.body.innerHTML).not.toContain("hunter2");
  });

  it("keeps the value when it didn't land, so it can be tried again", async () => {
    stubFetch({ status: status({ secret_wanted: "Password" }), post: () => ({ ok: false, error: "That field is gone." }) });
    const onDone = vi.fn();
    render(<SecretCard persona="assistant" name="Assistant" label="Password" onDone={onDone} />);
    fireEvent.change(screen.getByTestId("secret-card-input"), { target: { value: "hunter2" } });
    fireEvent.click(screen.getByTestId("secret-card-send"));
    await screen.findByText("That field is gone.");
    expect((screen.getByTestId("secret-card-input") as HTMLInputElement).value).toBe("hunter2");
    expect(onDone).not.toHaveBeenCalled();
  });

  it("saying no is its own call, and sends nothing typed", async () => {
    const calls = stubFetch({ status: status({ secret_wanted: "Password" }) });
    const onDone = vi.fn();
    render(<SecretCard persona="assistant" name="Assistant" label="Password" onDone={onDone} />);
    fireEvent.change(screen.getByTestId("secret-card-input"), { target: { value: "hunter2" } });
    fireEvent.click(screen.getByTestId("secret-card-decline"));
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("declined"));
    expect(calls.filter((c) => c.method === "POST").map((c) => c.body)).toEqual([{ decline: true }]);
  });

  it("closes itself when the browser is no longer asking", async () => {
    stubFetch({ status: status({}) });
    const onEnded = vi.fn();
    render(<SecretCard persona="assistant" name="Assistant" label="Password" onDone={vi.fn()} onEnded={onEnded} />);
    await waitFor(() => expect(onEnded).toHaveBeenCalled());
  });
});

describe("a request for the user's hands", () => {
  it("take control hands over the wheel, then opens the view", async () => {
    const calls = stubFetch({ status: status({ requested: true, reason: "Sign in" }), post: () => ({ ok: true }) });
    const onTaken = vi.fn();
    render(
      <ComputerHelpCard persona="assistant" name="Assistant" reason="Sign in" url="https://example.com/login" onTaken={onTaken} onDeclined={vi.fn()} />,
    );
    expect(screen.getByText("example.com")).toBeTruthy();
    fireEvent.click(screen.getByTestId("computer-help-take"));
    await waitFor(() => expect(onTaken).toHaveBeenCalledWith("assistant"));
    expect(calls.find((c) => c.method === "POST")?.body).toEqual({ action: "take" });
  });

  it("not now is an answer, not a wait", async () => {
    const calls = stubFetch({ status: status({ requested: true }), post: () => ({ ok: true }) });
    const onDeclined = vi.fn();
    render(<ComputerHelpCard persona="assistant" name="Assistant" reason="" url="" onTaken={vi.fn()} onDeclined={onDeclined} />);
    fireEvent.click(screen.getByTestId("computer-help-decline"));
    await waitFor(() => expect(onDeclined).toHaveBeenCalled());
    expect(calls.find((c) => c.method === "POST")?.body).toEqual({ action: "decline" });
  });

  it("closes itself once the request has lapsed", async () => {
    stubFetch({ status: status({ requested: false }) });
    const onEnded = vi.fn();
    render(<ComputerHelpCard persona="assistant" name="Assistant" reason="x" url="" onTaken={vi.fn()} onDeclined={vi.fn()} onEnded={onEnded} />);
    await waitFor(() => expect(onEnded).toHaveBeenCalled());
  });
});

describe("a kept screen", () => {
  it("draws the screen the turn kept, and opens the live view on request", async () => {
    stubFetch({ frame: { ok: true, data_url: "data:image/jpeg;base64,AAAA" } });
    const onWatch = vi.fn();
    render(<ComputerTile persona="assistant" frame="f-1-0001" url="https://example.com/a" title="Example" onWatch={onWatch} />);
    const img = (await screen.findByRole("img")) as HTMLImageElement;
    expect(img.src).toContain("data:image/jpeg");
    fireEvent.click(screen.getByTestId("computer-tile-watch"));
    expect(onWatch).toHaveBeenCalledWith("assistant");
  });

  it("says a deleted screen is gone rather than showing another", async () => {
    stubFetch({ frame: { ok: false, error: "That screen is no longer kept." } });
    await act(async () => {
      render(<ComputerTile persona="assistant" frame="f-1-0001" url="https://example.com/a" title="" />);
    });
    await screen.findByText("That screen is no longer kept.");
    expect(screen.queryByRole("img")).toBeNull();
  });
});
