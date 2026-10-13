import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Transcript } from "../Transcript";
import type { Item } from "../../types";

afterEach(cleanup);

// §33 TurnGroup: the user-message → final-answer span is ONE disclosure; interior assistant
// text is narration INSIDE it, the trailing assistant text is the answer OUTSIDE it; steps
// are humanized one-liners; approvals fold into their tool's row as a chip.
const TURN: Item[] = [
  { kind: "user", text: "post the digest" },
  { kind: "assistant", text: "Checking what merged since yesterday." },
  { kind: "tool", id: "t1", name: "read_file", args: { path: "docs/runbook.md" }, status: "ok" },
  { kind: "approval", name: "send_message", args: { target: "slack:T1/C9" }, reason: "", resolved: "once" },
  { kind: "tool", id: "t2", name: "send_message", args: { target: "slack:T1/C9", text: "hi" }, status: "ok", preview: '{"ok": true}' },
  { kind: "assistant", text: "Posted to #all-hands." },
];
describe("TurnGroup (Transcript §33)", () => {
  it("groups the whole turn; answer stays outside; narration and humanized steps inside", () => {
    const { container } = render(<Transcript items={TURN} onApprove={vi.fn()} />);

    // Collapsed at rest: "2 steps", NO approval count, and no step/narration content visible.
    expect(screen.getByText("2 steps")).toBeTruthy();
    expect(screen.queryByText(/approval/)).toBeNull();
    expect(screen.queryByTestId("turn-narration")).toBeNull();
    expect(screen.queryByText(/Sent a Slack message/)).toBeNull();

    // The final answer is a normal bubble OUTSIDE the disclosure, visible while collapsed.
    expect(screen.getByText("Posted to #all-hands.")).toBeTruthy();

    // Expand → narration renders quiet inside; steps are English lines, not raw args;
    // the approval is a chip on the send_message row, not a separate box.
    fireEvent.click(container.querySelector("summary.stepgroup-head")!);
    expect(screen.getByTestId("turn-narration").textContent).toContain("Checking what merged");
    expect(screen.getByText("runbook.md")).toBeTruthy();
    expect(screen.getByText(/Sent a Slack message to/)).toBeTruthy();
    expect(screen.getByText("✓ approved")).toBeTruthy();
    expect(screen.queryByText("send_message approval")).toBeNull();

    // Raw stays one click away: the row's Details toggle reveals args + result verbatim.
    fireEvent.click(screen.getAllByText("Details")[1]);
    expect(container.textContent).toContain('{"ok": true}');
  });

  // A call the run never heard back from was cut off, not broken. It used to render in
  // danger red with the word "stopped" — a failure claim the Activity tab refused to make.
  it("a stopped step is neutral, not a failure", () => {
    const items: Item[] = [
      { kind: "user", text: "list the files" },
      { kind: "tool", id: "t1", name: "run_shell", args: { command: "ls" }, status: "stopped" },
      { kind: "assistant", text: "Stopped." },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    fireEvent.click(container.querySelector("summary.stepgroup-head")!);
    const row = screen.getByTestId("turn-step");
    expect(screen.getByTestId("step-stopped").className).toContain("text-faint");
    expect(row.querySelector(".text-danger")).toBeNull();
  });

  it("a running turn is labeled Running but starts COLLAPSED (§33 ref #3)", () => {
    const items: Item[] = [
      { kind: "assistant", text: "Looking at the repo." },
      { kind: "tool", id: "t1", name: "grep", args: { pattern: "TODO" }, status: "…" },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(screen.getByText(/Running 1 step…/)).toBeTruthy();
    expect(screen.queryByTestId("turn-narration")).toBeNull(); // collapsed by default
    expect(screen.getByTestId("turn-live-line").textContent).toContain("Looking at the repo");
    fireEvent.click(container.querySelector("summary.stepgroup-head")!);
    expect(screen.getByTestId("step-running")).toBeTruthy();
  });

  it("a finished turn stops spinning: settled steps drop the Running label and the spinners", () => {
    // The bug behind this: steps only ever announced that they had STARTED, so the group
    // kept deriving `running` from them and painted live spinners under a printed answer.
    const items: Item[] = [
      { kind: "tool", id: "t1", name: "todo_write", args: {}, status: "ok" },
      { kind: "tool", id: "t2", name: "write_file", args: { path: "latency-check.txt" }, status: "ok" },
      { kind: "assistant", text: "I created latency-check.txt." },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(screen.queryByText(/Running/)).toBeNull();
    expect(screen.getByText(/2 steps/)).toBeTruthy();
    fireEvent.click(container.querySelector("summary.stepgroup-head")!);
    expect(screen.queryByTestId("step-running")).toBeNull();
  });

  it("declined approvals keep their own 'Wanted to' row and surface on the collapsed line", () => {
    const items: Item[] = [
      { kind: "tool", id: "t1", name: "read_file", args: { path: "a.md" }, status: "ok" },
      { kind: "approval", name: "run_shell", args: { command: "rm -rf build/" }, reason: "", resolved: "deny" },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(screen.getByTestId("stepgroup-declined").textContent).toBe("1 declined");
    fireEvent.click(container.querySelector("summary.stepgroup-head")!);
    const ask = screen.getByTestId("turn-ask");
    expect(ask.textContent).toContain("Wanted to run");
    expect(ask.textContent).toContain("rm -rf build/");
    expect(ask.textContent).toContain("✕ declined");
  });

  it("assistant-only turns stay plain bubbles (no disclosure)", () => {
    const items: Item[] = [
      { kind: "user", text: "hi" },
      { kind: "assistant", text: "Hello there." },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(container.querySelector("details.stepgroup")).toBeNull();
    expect(screen.getByText("Hello there.")).toBeTruthy();
  });
});

describe("live turns (§33 flicker fix)", () => {
  const LIVE: Item[] = [
    { kind: "user", text: "build the app" },
    { kind: "tool", id: "t1", name: "read_file", args: { path: "data.json" }, status: "ok" },
    { kind: "assistant", text: "Inspecting the fetched dataset next." },
  ];

  it("while running, trailing assistant text stays INSIDE the group — no answer bubble flash", () => {
    const { container } = render(<Transcript items={LIVE} onApprove={vi.fn()} running />);
    // No assistant bubble anywhere; the group starts COLLAPSED with the narration riding
    // the header as the live line (§33 ref #3 — expanding is opt-in).
    expect(container.querySelector(".bubble-assistant")).toBeNull();
    expect(screen.queryByTestId("turn-narration")).toBeNull();
    expect(screen.getByTestId("turn-live-line").textContent).toContain("Inspecting the fetched dataset");
    // Expanding shows it as the quiet line inside.
    fireEvent.click(container.querySelector("summary.stepgroup-head")!);
    expect(screen.getByTestId("turn-narration").textContent).toContain("Inspecting the fetched dataset");
    // Once the turn ends (running=false), the same trailing text IS the answer bubble.
    cleanup();
    const done = render(<Transcript items={LIVE} onApprove={vi.fn()} />);
    expect(done.container.querySelector(".bubble-assistant")?.textContent).toContain(
      "Inspecting the fetched dataset",
    );
  });

  it("quiet streamed text rides the collapsed header and the expanded body — never floats", () => {
    const { container } = render(
      <Transcript
        items={LIVE}
        onApprove={vi.fn()}
        running
        streamingText="The quote endpoint rate-limited, so I'm checking the historical pages."
      />,
    );
    // Collapsed: the STREAMING text wins the header live line (fresher than the last item).
    expect(screen.getByTestId("turn-live-line").textContent).toContain("quote endpoint rate-limited");
    expect(container.querySelector(".bubble-assistant")).toBeNull();
    // Expanded: it renders as the small quiet line under the steps.
    fireEvent.click(container.querySelector("summary.stepgroup-head")!);
    expect(screen.getByTestId("turn-live-stream").textContent).toContain("quote endpoint rate-limited");
  });

  it("a PENDING approval neither splits the turn nor promotes the narration", () => {
    const items: Item[] = [
      ...LIVE,
      { kind: "approval", name: "write_file", args: { path: "app.html" }, reason: "" }, // unresolved
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} running />);
    expect(container.querySelectorAll("details.stepgroup")).toHaveLength(1);
    expect(container.querySelector(".bubble-assistant")).toBeNull();
  });

  it("a live run with NO tool activity is a plain streaming reply — bubbles as ever", () => {
    const items: Item[] = [
      { kind: "user", text: "hi" },
      { kind: "assistant", text: "Hello!" },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} running />);
    expect(container.querySelector("details.stepgroup")).toBeNull();
    expect(container.querySelector(".bubble-assistant")?.textContent).toContain("Hello!");
  });

  // Another assistant answered (an `@` mention): its name sits above the reply, and
  // the conversation's own replies carry no label.
  it("labels a reply another assistant gave, and only that one", () => {
    const items: Item[] = [
      { kind: "user", text: "@expenses what did we spend?" },
      { kind: "assistant", text: "£412 this week.", persona: "expenses" },
      { kind: "user", text: "thanks" },
      { kind: "assistant", text: "Noted." },
    ];
    const names = (id: string) => (id === "expenses" ? "Expense Manager" : id);
    render(<Transcript items={items} onApprove={vi.fn()} personaName={names} />);
    const labels = screen.getAllByTestId("reply-from");
    expect(labels.map((l) => l.textContent)).toEqual(["Expense Manager"]);
  });
});
