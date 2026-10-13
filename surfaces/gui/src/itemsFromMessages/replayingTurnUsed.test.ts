import { describe, expect, it } from "vitest";
import { itemsFromMessages, mergeJournalSteps } from "../itemsFromMessages";
import type { ToolCall } from "../components/ActivityTab";


describe("replaying a turn that used a tool", () => {
  // The stored shape: one assistant message carrying both the answer (`content`)
  // and the calls it made (`items`). Order matters on the way out, because
  // `Transcript` finds the final answer positionally.
  const stored = [
    { role: "user", content: "chart the AI trends" },
    {
      role: "assistant",
      content: "I've researched AI trends and drawn the chart above.",
      items: [
        { kind: "tool", name: "chart", args: { spec: { type: "bar", x: { field: "a" }, y: { field: "b" }, data: [{ a: "x", b: 1 }] }, source: "s" }, status: "ok" },
      ],
    },
  ] as any;

  it("puts the work before the answer, not after it", () => {
    // `Transcript.flush` pops TRAILING assistant items to find the turn's answer.
    // Emitted ahead of its tools, the answer never trails — so it was folded into
    // the collapsed "N steps" disclosure and the conversation came back with its
    // own reply hidden behind a triangle.
    const items = itemsFromMessages(stored);
    const kinds = items.map((i) => i.kind);
    expect(kinds).toEqual(["user", "tool", "assistant"]);
    expect(items[items.length - 1].kind).toBe("assistant");
  });
});

describe("mergeJournalSteps", () => {
  // The store keeps the answer; the run journal keeps the last turn's working. A reopened
  // conversation read as though the model had simply known, because only the side panel
  // ever read the journal.
  const call = (over: Partial<ToolCall>): ToolCall => ({
    seq: 1,
    at: 0,
    name: "run_shell",
    title: "ls",
    args: { command: "ls" },
    status: "ok",
    result: "alpha.md",
    pending: false,
    ...over,
  });

  it("puts the last turn's calls between the question and the answer", () => {
    const items = itemsFromMessages([
      { role: "user", content: "earlier" },
      { role: "assistant", content: "earlier answer" },
      { role: "user", content: "list the files with run_shell" },
      { role: "assistant", content: "The files are: alpha.md" },
    ] as any);
    const out = mergeJournalSteps(items, [call({ seq: 3 })]);
    expect(out.map((i) => i.kind)).toEqual(["user", "assistant", "user", "tool", "assistant"]);
    expect(out[3]).toMatchObject({ kind: "tool", name: "run_shell", status: "ok", preview: "alpha.md" });
  });

  it("goes ahead of a notice that ended the turn, too", () => {
    const items: any[] = [
      { kind: "user", text: "q" },
      { kind: "notice", tone: "warn", text: "Error: model down", retriable: true },
    ];
    expect(mergeJournalSteps(items, [call({})]).map((i) => i.kind)).toEqual(["user", "tool", "notice"]);
  });

  it("leaves a turn that already has steps alone, so nothing draws twice", () => {
    const items: any[] = [
      { kind: "user", text: "chart it" },
      { kind: "tool", id: "c", name: "chart", args: {}, status: "ok" },
      { kind: "assistant", text: "Q4 carried the year." },
    ];
    expect(mergeJournalSteps(items, [call({})])).toBe(items);
  });

  it("does nothing for a question with no reply yet", () => {
    const items: any[] = [{ kind: "user", text: "q" }];
    expect(mergeJournalSteps(items, [call({})])).toBe(items);
  });

  it("marks calls the run never heard back from as stopped, not running", () => {
    const items: any[] = [
      { kind: "user", text: "q" },
      { kind: "assistant", text: "a" },
    ];
    const out = mergeJournalSteps(items, [call({ pending: true, status: undefined }), call({ seq: 2, stopped: true })]);
    expect(out.filter((i) => i.kind === "tool").map((t: any) => t.status)).toEqual(["stopped", "stopped"]);
  });

  it("returns the same array when the journal has no calls", () => {
    const items: any[] = [{ kind: "user", text: "q" }, { kind: "assistant", text: "a" }];
    expect(mergeJournalSteps(items, [])).toBe(items);
  });
});
