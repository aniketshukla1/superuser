import { describe, expect, it } from "vitest";
import { itemsFromMessages, settleRunningTools } from "../itemsFromMessages";


describe("itemsFromMessages _display sidecar", () => {
  it("attaches hidden counts to the matching tool item", () => {
    const items = itemsFromMessages([
      { role: "user", content: "check my mail" },
      {
        role: "assistant",
        content: "",
        tool_calls: [
          { id: "t1", function: { name: "gmail_search_messages", arguments: "{}" } },
          { id: "t2", function: { name: "gmail_get_message", arguments: "{}" } },
        ],
      },
      {
        role: "tool",
        tool_call_id: "t1",
        content: '{"ok": true, "data": {"messages": []}}',
        _display: { hidden_by_filters: 2, connector: "gmail" },
      },
      { role: "tool", tool_call_id: "t2", content: '{"ok": true}' },
    ] as any);

    const tools = items.filter((i: any) => i.kind === "tool") as any[];
    expect(tools).toHaveLength(2);
    expect(tools[0].hidden).toBe(2);
    expect(tools[1].hidden).toBeUndefined();
    expect(tools[0].preview).not.toContain("hidden"); // content stays clean
  });
});

describe("itemsFromMessages timestamps", () => {
  it("carries the server ts through to user/assistant items; pre-stamp history gets none", () => {
    const items = itemsFromMessages([
      { role: "user", content: "hi", ts: 1752969720 },
      { role: "assistant", content: "hello", ts: 1752969724 },
      { role: "user", content: "old message" }, // saved before the server stamped ts
    ] as any);

    expect(items).toEqual([
      { kind: "user", text: "hi", ts: 1752969720 },
      { kind: "assistant", text: "hello", ts: 1752969724 },
      { kind: "user", text: "old message" },
    ]);
  });

  // An `@` mention's reply carries who gave it; the conversation's own replies don't.
  it("carries who answered a mention, and nothing on an ordinary reply", () => {
    const items = itemsFromMessages([
      { role: "assistant", content: "£412 this week.", persona: "expenses" },
      { role: "assistant", content: "Noted." },
    ] as any);
    expect(items).toEqual([
      { kind: "assistant", text: "£412 this week.", persona: "expenses" },
      { kind: "assistant", text: "Noted." },
    ]);
  });
});

describe("itemsFromMessages notices", () => {
  it("replays persisted error/interrupted markers; only errors are retriable", () => {
    const items = itemsFromMessages([
      { role: "user", content: "hi" },
      { role: "assistant", content: "partial ans" },
      { role: "notice", kind: "interrupted", ts: 1752969720 },
      { role: "user", content: "again" },
      { role: "notice", kind: "error", text: "model down", ts: 1752969724 },
    ] as any);

    expect(items).toEqual([
      { kind: "user", text: "hi" },
      { kind: "assistant", text: "partial ans" },
      { kind: "notice", tone: "warn", text: "Interrupted." },
      { kind: "user", text: "again" },
      { kind: "notice", tone: "warn", text: "Error: model down", retriable: true },
    ]);
  });
});

describe("itemsFromMessages model switch", () => {
  it("replays the persisted model_switch marker as an info notice", () => {
    const items = itemsFromMessages([
      { role: "user", content: "hi" },
      { role: "notice", kind: "model_switch", text: "Model switched to Kimi K2.6 · Moonshot" },
    ] as any);
    expect(items[1]).toEqual({
      kind: "notice",
      tone: "info",
      text: "Model switched to Kimi K2.6 · Moonshot",
    });
  });
});

describe("itemsFromMessages compaction", () => {
  it("replays the persisted compacted marker as an info notice (the divider)", () => {
    const items = itemsFromMessages([
      { role: "user", content: "hi" },
      { role: "notice", kind: "compacted", text: "Context compacted — earlier turns were summarized" },
    ] as any);
    expect(items[1]).toEqual({
      kind: "notice",
      tone: "info",
      text: "Context compacted — earlier turns were summarized",
    });
  });
});

describe("itemsFromMessages reasoning", () => {
  it("attaches the reasoning sidecar to assistant items; thinking-only messages still render", () => {
    const items = itemsFromMessages([
      { role: "user", content: "hi" },
      { role: "assistant", content: "answer", reasoning: "let me think" },
      { role: "assistant", content: "", reasoning: "stopped mid-thought" },
    ] as any);
    expect(items[1]).toEqual({ kind: "assistant", text: "answer", reasoning: "let me think" });
    expect(items[2]).toEqual({ kind: "assistant", text: "", reasoning: "stopped mid-thought" });
  });
});

describe("settleRunningTools", () => {
  const running = (name: string) => ({ kind: "tool", id: name, name, args: {}, status: "…" }) as any;

  it("settles steps whose completion never arrived, leaving reported ones alone", () => {
    const items = [
      { kind: "user", text: "make a file" },
      { kind: "tool", id: "a", name: "write_file", args: {}, status: "ok" },
      running("todo_write"),
      { kind: "assistant", text: "done" },
    ] as any;

    const out = settleRunningTools(items, "ok");
    const statuses = out.map((it) => (it.kind === "tool" ? it.status : null));
    expect(statuses).not.toContain("…");
    expect(statuses[1]).toBe("ok"); // already-reported step untouched
    expect(statuses[2]).toBe("ok");
    expect(out[0]).toBe(items[0]); // non-tool items pass through by reference
  });

  it("carries the terminal reason, so a stopped step doesn't read as a success", () => {
    const out = settleRunningTools([running("run_command")] as any, "stopped");
    expect((out[0] as any).status).toBe("stopped");
  });

  it("returns the same array when nothing is running, so React can skip the render", () => {
    const items = [{ kind: "tool", id: "a", name: "write_file", args: {}, status: "ok" }] as any;
    expect(settleRunningTools(items, "ok")).toBe(items);
  });
});

describe("charts survive a reload", () => {
  // The store keeps the final answer and nothing else, so a chart drawn during the
  // turn had no way back onto the screen — while the answer above it went on saying
  // "as shown". The socket persists the call on the assistant message; this reads it.
  const SPEC = { type: "bar", y: { field: "v" }, data: [{ k: "a", v: 1 }] };

  it("replays a persisted chart call as a step the transcript can draw", () => {
    const items = itemsFromMessages([
      { role: "user", content: "chart it" },
      {
        role: "assistant",
        content: "Q4 carried the year.",
        items: [
          { kind: "todo", content: "read the file", status: "done" },
          { kind: "tool", name: "chart", args: { path: "r.chart.json", spec: SPEC, source: "q.xlsx" }, status: "ok" },
        ],
      },
    ] as any);
    const tool = items.find((i) => i.kind === "tool");
    expect(tool).toBeTruthy();
    expect((tool as any).name).toBe("chart");
    expect((tool as any).args.spec.data).toHaveLength(1);
    expect((tool as any).args.source).toBe("q.xlsx");
    expect((tool as any).status).toBe("ok");
  });

  it("leaves the other item kinds to their own handlers", () => {
    const items = itemsFromMessages([
      {
        role: "assistant",
        content: "done",
        items: [
          { kind: "todo", content: "x", status: "done" },
          { kind: "thread", session_id: "s2", title: "Sub" },
        ],
      },
    ] as any);
    expect(items.some((i) => i.kind === "tool")).toBe(false);
    expect(items.some((i) => i.kind === "threads")).toBe(true);
  });
});
