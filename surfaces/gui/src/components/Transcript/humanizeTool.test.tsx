import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Transcript } from "../Transcript";
import { humanizeTool } from "../../humanize";
import type { Item } from "../../types";

afterEach(cleanup);
describe("humanizeTool", () => {
  it("prefers run_shell's model-written description and keeps the command as the object", () => {
    const line = humanizeTool("run_shell", { command: "git log --since=yesterday", description: "List yesterday's merges" });
    expect(line.pre).toBe("Ran ");
    expect(line.obj).toBe("git log --since=yesterday");
    expect(line.post).toContain("list yesterday's merges");
  });

  it("falls back to 'Used <tool> — <short args>' for unknown tools", () => {
    const line = humanizeTool("gmail_search_messages", { query: "from:ci" });
    expect(line.pre).toBe("Used gmail_search_messages");
    expect(line.post).toContain("query=from:ci");
  });

  it("summarizes todo_write by its single item and status", () => {
    const line = humanizeTool("todo_write", { todos: [{ content: "Post the digest", status: "in_progress" }] });
    expect(line.pre).toBe("Updated the plan — ");
    expect(line.obj).toContain("Post the digest");
    expect(line.post).toBe(" → in progress");
  });

  it("still renders pre-rename todo_write histories (legacy `items` key)", () => {
    const line = humanizeTool("todo_write", { items: [{ content: "Old plan", status: "pending" }] });
    expect(line.obj).toContain("Old plan");
  });
});

// The transcript carries two rhythms: blocks inside one exchange sit close, and
// the space that separates one exchange from the next goes on the block that
// STARTS the next one. A single uniform gap put a question as far from its own
// answer as from an unrelated turn.
describe("Transcript rhythm", () => {
  const TWO_EXCHANGES: Item[] = [
    { kind: "user", text: "first question" },
    { kind: "assistant", text: "first answer" },
    { kind: "user", text: "second question" },
    { kind: "assistant", text: "second answer" },
  ];

  it("separates exchanges, not the blocks inside one", () => {
    const { container } = render(<Transcript items={TWO_EXCHANGES} onApprove={vi.fn()} />);
    const starts = container.querySelectorAll(".exchange-start");

    // One marker, on the SECOND question. The first opens the transcript and has
    // nothing above it to be separated from; the answers belong to the question
    // above them and must not be pushed away from it.
    expect(starts.length).toBe(1);
    expect(starts[0].textContent).toContain("second question");
  });

  it("puts the space on a direct child, or the rule that applies it never matches", () => {
    const { container } = render(<Transcript items={TWO_EXCHANGES} onApprove={vi.fn()} />);
    const transcript = container.querySelector(".transcript")!;
    const start = container.querySelector(".exchange-start")!;
    // `.transcript > .exchange-start` is a child selector: if the bubble is ever
    // wrapped in anything, the spacing silently stops applying and the two rhythms
    // collapse back into one with nothing failing.
    expect(start.parentElement).toBe(transcript);
  });

  // Dated rules across the transcript are gone (owner ask): a rule labelled "Today" drawn
  // across a conversation you opened today divides nothing, and the sidebar already says
  // which conversation this is and when it was last touched.
  it("draws no dated rules, and still breaks between exchanges across days", () => {
    const day = (d: string) => new Date(d).getTime() / 1000;
    const acrossDays: Item[] = [
      { kind: "user", text: "monday question", ts: day("2026-08-10T09:00:00Z") },
      { kind: "assistant", text: "monday answer", ts: day("2026-08-10T09:01:00Z") },
      { kind: "user", text: "tuesday question", ts: day("2026-08-11T09:00:00Z") },
    ];
    const { container } = render(<Transcript items={acrossDays} onApprove={vi.fn()} />);
    expect(container.querySelectorAll(".day-sep").length).toBe(0);
    expect(screen.queryByText("Today")).toBeNull();
    expect(screen.queryByText("Yesterday")).toBeNull();
    // The separator used to BE the break between those exchanges, and suppressed the
    // margin. With it gone the margin has to do that job, or a new day would run straight
    // on from the previous one.
    expect(container.querySelectorAll(".exchange-start").length).toBe(1);
  });

  // The caption said "ASSISTANT" above every reply. Your questions are right-aligned in a
  // box and its replies are left-aligned and bare — the label restated the layout, once per
  // turn, down the whole transcript.
  it("does not caption the assistant's replies", () => {
    const { container } = render(
      <Transcript
        items={[
          { kind: "user", text: "q" },
          { kind: "assistant", text: "a" },
        ]}
        onApprove={vi.fn()}
      />,
    );
    expect(container.querySelectorAll(".bubble-assistant .who").length).toBe(0);
    expect(screen.queryByText(/^assistant$/i)).toBeNull();
    // …but the reply itself is still there.
    expect(screen.getByText("a")).toBeTruthy();
  });
});

describe("the row under a reply", () => {
  // The thumbs were rendered after the action row, but the row is laid over the space under
  // the bubble — so the two sets of controls occupied the same spot and the arrows landed
  // between copy, read aloud and retry. They belong IN the row.
  it("puts the verdict in the same row as copy and retry, and the reasons under the reply", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        async () =>
          ({
            ok: true,
            json: async () => ({ reasons: [{ id: "made_it_up", label: "It made something up" }], given: {} }),
          }) as Response,
      ),
    );
    const items: Item[] = [
      { kind: "user", text: "what changed?" },
      { kind: "assistant", text: "Two files.", ts: 1_800_000_000 },
    ];
    render(<Transcript items={items} sessionId="s1" onRetry={vi.fn()} onApprove={vi.fn()} />);
    // The user's own message has a copy row too; the reply's is the one inside .bubble-assistant.
    await screen.findByLabelText("This was good");
    const row = document.querySelector(".bubble-assistant [data-testid='bubble-copy']")!.parentElement!;
    expect(row.contains(screen.getByLabelText("This was good"))).toBe(true);
    expect(row.contains(screen.getByTestId("bubble-retry"))).toBe(true);

    fireEvent.click(screen.getByLabelText("Something was wrong"));
    const reasons = await screen.findByRole("group", { name: "What was wrong?" });
    expect(row.contains(reasons)).toBe(false);
    vi.unstubAllGlobals();
  });
});
