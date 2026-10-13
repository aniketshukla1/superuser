import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { BoundariesSection } from "./BoundariesSection";

// The page's promises, each one a test: a preset says what it costs before it is
// switched on; a rule that can't be read is said so beside the rule and is never
// saved; "Test first" replays the DRAFT, not what is saved; and an unreadable file is
// named as the reason everything is being refused, with the way out beside it.

const PRESETS = [
  { id: "never-send", name: "Never send messages", when: 'intent == "send"', cost: "Assistants can still write the message. You send it yourself." },
  { id: "apps-read-only", name: "Connected apps are read-only", when: 'intent == "write_tool"', cost: "They can search your apps but can't change anything in them." },
];
const FIELDS = [{ field: "intent", means: "What the action does" }];

type Call = { url: string; method?: string; body: any };

function stubFetch(opts: {
  boundaries?: any;
  problem?: string | null;
  save?: (body: any) => any;
  check?: (when: string) => string | null;
  test?: any;
} = {}) {
  const calls: Call[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    const u = String(url);
    const body = init?.body ? JSON.parse(String(init.body)) : null;
    calls.push({ url: u, method: init?.method, body });
    const reply = (v: any) => ({ ok: true, json: async () => v }) as Response;
    if (u.endsWith("/v1/boundaries/check")) {
      const problem = opts.check ? opts.check(body.when) : null;
      return reply({ ok: problem === null, problem });
    }
    if (u.endsWith("/v1/boundaries/test")) return reply({ ok: true, report: opts.test, problems: [] });
    if (u.endsWith("/v1/boundaries/reset")) return reply({ ok: true, boundaries: { mode: "enforce", deny: [], allow: ["true"] } });
    if (u.endsWith("/v1/boundaries") && init?.method === "POST") {
      return reply(opts.save ? opts.save(body) : { ok: true, boundaries: body });
    }
    if (u.endsWith("/v1/boundaries")) {
      return reply({
        boundaries: opts.boundaries ?? { mode: "enforce", deny: [], allow: ["true"] },
        problem: opts.problem ?? null,
        problems: [],
        presets: PRESETS,
        fields: FIELDS,
      });
    }
    return reply({});
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe("BoundariesSection", () => {
  it("adds a preset with its cost shown, and saves exactly that rule", async () => {
    const calls = stubFetch();
    render(<BoundariesSection />);
    await screen.findByTestId("boundary-presets");
    // The cost is on screen before anything is switched on.
    expect(screen.getByText(/You send it yourself/)).toBeTruthy();

    fireEvent.click(screen.getByTestId("boundary-preset-never-send"));
    expect(screen.getByTestId("boundary-rule-cost").textContent).toContain("You send it yourself");
    expect((screen.getByTestId("boundary-preset-never-send") as HTMLButtonElement).disabled).toBe(true);

    fireEvent.click(screen.getByTestId("boundaries-save"));
    await waitFor(() => expect(screen.getByTestId("boundaries-status").textContent).toContain("applies to the next action"));
    const post = calls.find((c) => c.method === "POST" && c.url.endsWith("/v1/boundaries"));
    expect(post!.body).toEqual({
      mode: "enforce",
      deny: [{ name: "Never send messages", when: 'intent == "send"', preset: "never-send" }],
      allow: ["true"],
    });
  });

  it("says what is wrong with a rule beside it, and a refused save is not reported as saved", async () => {
    stubFetch({
      check: (when) => (when.includes("elemnt") ? "There's no field called `elemnt.name` — did you mean `element.name`?" : null),
      save: () => ({ ok: false, problems: [{ list: "deny", index: 0, message: "This rule can't be read at character 8: write `==`." }] }),
    });
    render(<BoundariesSection />);
    await screen.findByTestId("boundary-presets");

    fireEvent.click(screen.getByTestId("boundary-add-blank"));
    fireEvent.change(screen.getByTestId("boundary-rule-when"), { target: { value: 'elemnt.name == "x"' } });
    expect(await screen.findByTestId("boundary-rule-problem", {}, { timeout: 2000 })).toBeTruthy();
    expect(screen.getByTestId("boundary-rule-problem").textContent).toContain("did you mean `element.name`");
    // A broken rule would refuse everything it might cover, so it can't be tested yet.
    expect((screen.getByTestId("boundaries-test-run") as HTMLButtonElement).disabled).toBe(true);

    fireEvent.change(screen.getByTestId("boundary-rule-when"), { target: { value: 'intent = "send"' } });
    await waitFor(() => expect(screen.queryByTestId("boundary-rule-problem")).toBeNull(), { timeout: 2000 });
    fireEvent.click(screen.getByTestId("boundaries-save"));
    await waitFor(() => expect(screen.getByTestId("boundaries-status").textContent).toContain("Not saved"));
    expect(screen.getByTestId("boundary-rule-problem").textContent).toContain("write `==`");
  });

  it("an empty rule is caught before it reaches the server", async () => {
    const calls = stubFetch();
    render(<BoundariesSection />);
    await screen.findByTestId("boundary-presets");
    fireEvent.click(screen.getByTestId("boundary-add-blank"));
    fireEvent.click(screen.getByTestId("boundaries-save"));
    expect((await screen.findByTestId("boundary-rule-problem")).textContent).toContain("Write the rule");
    expect(calls.some((c) => c.method === "POST" && c.url.endsWith("/v1/boundaries"))).toBe(false);
  });

  it("Test first replays the unsaved draft and says what it would change", async () => {
    const calls = stubFetch({
      test: {
        scanned: 40,
        would_refuse: 2,
        would_allow: 0,
        unchanged: 38,
        changes: [
          { at: 1757000000, session: "s1", persona: "assistant", tool: "send_message", intent: "send", what: "sending a message to slack:#ops", was: "allowed", would: "refused", rule: 'intent == "send"', rule_name: "Never send messages" },
        ],
      },
    });
    render(<BoundariesSection />);
    await screen.findByTestId("boundary-presets");
    fireEvent.click(screen.getByTestId("boundary-preset-never-send"));
    fireEvent.click(screen.getByTestId("boundaries-test-run"));

    const report = await screen.findByTestId("boundaries-report");
    expect(report.textContent).toContain("Tested against the last 40 actions: 2 actions that went ahead would be refused.");
    expect(screen.getByTestId("boundaries-change").textContent).toContain("sending a message to slack:#ops");
    const post = calls.find((c) => c.url.endsWith("/v1/boundaries/test"));
    expect(post!.body.boundaries.deny[0].when).toBe('intent == "send"');
    // Testing saved nothing.
    expect(calls.some((c) => c.method === "POST" && c.url.endsWith("/v1/boundaries"))).toBe(false);
  });

  it("names an unreadable file as the reason for refusing everything, with the way out", async () => {
    const calls = stubFetch({
      problem: "boundaries.json isn't valid (expected value at line 1 column 3)",
      boundaries: { mode: "enforce", deny: [{ name: "Your boundaries file can't be read", when: "true" }], allow: [] },
    });
    vi.stubGlobal("confirm", () => true);
    render(<BoundariesSection />);
    const banner = await screen.findByTestId("boundaries-unreadable");
    expect(banner.textContent).toContain("every action is being refused");
    expect(banner.textContent).toContain("isn't valid");
    // The server's backstop rule is not presented as one the user wrote.
    expect(screen.queryByTestId("boundary-rule")).toBeNull();

    fireEvent.click(screen.getByText("Reset to no boundaries"));
    await waitFor(() => expect(calls.some((c) => c.url.endsWith("/v1/boundaries/reset"))).toBe(true));
  });

  it("switching to record mode says nothing will be stopped", async () => {
    stubFetch();
    render(<BoundariesSection />);
    await screen.findByTestId("boundary-presets");
    fireEvent.click(screen.getByTestId("boundaries-mode-record"));
    expect(screen.getByText(/Nothing is stopped/)).toBeTruthy();
  });
});
