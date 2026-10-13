import { describe, expect, it } from "vitest";
import { clampSplit, filesTouchedBy, rawText, toolCallsFrom } from "./ActivityTab";
import type { RunEvent } from "../api";

let seq = 0;
const ev = (type: string, data: Record<string, unknown>): RunEvent => ({ seq: seq++, type, at: 0, data });

describe("reading a run journal as work rather than frames", () => {
  it("joins a tool's two halves into one call", () => {
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "run_shell", arguments: { command: "npm test" } }),
      ev("tool_finished", { name: "run_shell", status: "ok", result_preview: "12 passed" }),
    ]);
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ name: "run_shell", title: "npm test", result: "12 passed", pending: false });
  });

  it("leaves a call that hasn't finished marked as running", () => {
    const calls = toolCallsFrom([ev("tool_proposed", { name: "run_shell", arguments: { command: "sleep 30" } })]);
    expect(calls[0].pending).toBe(true);
  });

  it("matches finishes to calls in order when a tool is used twice", () => {
    // Both frames say only "write_file", so the second finish must not settle the
    // first call — otherwise one row shows the wrong output and one spins forever.
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "write_file", arguments: { path: "a.md" } }),
      ev("tool_proposed", { name: "write_file", arguments: { path: "b.md" } }),
      ev("tool_finished", { name: "write_file", status: "ok", result_preview: "wrote a" }),
      ev("tool_finished", { name: "write_file", status: "ok", result_preview: "wrote b" }),
    ]);
    expect(calls.map((c) => [c.title, c.result])).toEqual([
      ["a.md", "wrote a"],
      ["b.md", "wrote b"],
    ]);
  });

  it("titles each tool by the argument that identifies it", () => {
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "grep", arguments: { pattern: "TODO" } }),
      ev("tool_proposed", { name: "web_fetch", arguments: { url: "https://example.com" } }),
      ev("tool_proposed", { name: "read_file", arguments: { path: "src/main.rs" } }),
    ]);
    expect(calls.map((c) => c.title)).toEqual(["TODO", "https://example.com", "src/main.rs"]);
  });

  it("ignores frames that aren't tool calls", () => {
    const calls = toolCallsFrom([
      ev("turn_start", {}),
      ev("assistant_delta", { text: "thinking" }),
      ev("tool_proposed", { name: "list_dir", arguments: { path: "." } }),
      ev("turn_done", { artifacts: [] }),
    ]);
    expect(calls).toHaveLength(1);
  });

  it("marks a failed call so it doesn't read as a success", () => {
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "run_shell", arguments: { command: "false" } }),
      ev("tool_finished", { name: "run_shell", status: "error", result_preview: "exit 1" }),
    ]);
    expect(calls[0].status).toBe("error");
  });

  it("survives a finish with no matching proposal", () => {
    // The journal is capped, so a long run can lose the opening half of a call.
    expect(() => toolCallsFrom([ev("tool_finished", { name: "run_shell", status: "ok" })])).not.toThrow();
  });
});

describe("which files a run changed", () => {
  it("counts writes and edits, not reads", () => {
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "read_file", arguments: { path: "read-only.md" } }),
      ev("tool_proposed", { name: "write_file", arguments: { path: "made.md" } }),
      ev("tool_proposed", { name: "edit_file", arguments: { path: "changed.md" } }),
    ]);
    expect(filesTouchedBy(calls)).toEqual(["made.md", "changed.md"]);
  });

  it("lists a file edited repeatedly only once", () => {
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "edit_file", arguments: { path: "same.md" } }),
      ev("tool_proposed", { name: "edit_file", arguments: { path: "same.md" } }),
    ]);
    expect(filesTouchedBy(calls)).toEqual(["same.md"]);
  });
});

describe("the raw half shows the frame in full", () => {
  it("does not clip a long line", () => {
    // The old renderer sliced to 200 characters and then let CSS ellipsise what
    // survived, so a long result was unreadable in the one view meant to be honest.
    const long = "x".repeat(500);
    expect(rawText(ev("assistant_delta", { text: long }))).toBe(long);
  });

  it("keeps newlines rather than flattening them to one line", () => {
    const out = rawText(ev("tool_finished", { text: "line one\nline two" }));
    expect(out).toBe("line one\nline two");
  });

  it("pretty-prints a frame that carries no field it recognises", () => {
    const out = rawText(ev("turn_done", { artifacts: ["a.md"] }));
    expect(out).toContain("artifacts");
    expect(out).toContain("a.md");
  });

  it("gives an empty frame an empty line instead of a stray {}", () => {
    expect(rawText(ev("turn_start", {}))).toBe("");
  });

  it("renders a frame whose data is cyclic instead of throwing mid-render", () => {
    const cyclic: Record<string, unknown> = { kind: 1 };
    cyclic.self = cyclic;
    expect(() => rawText(ev("weird", cyclic))).not.toThrow();
  });
});

describe("the divider between the two halves", () => {
  it("keeps either half from collapsing to an ungrabbable sliver", () => {
    expect(clampSplit(0)).toBe(15);
    expect(clampSplit(100)).toBe(85);
  });

  it("passes a sane split through untouched", () => {
    expect(clampSplit(50)).toBe(50);
  });

  it("falls back rather than persisting NaN from a corrupt stored value", () => {
    expect(clampSplit(Number.NaN)).toBe(50);
  });
});

describe("a turn the user stopped", () => {
  it("does not leave a cut-off tool spinning forever", () => {
    // Reported: stop the chat and the Activity tab goes on showing tools as if
    // they were still being processed. A tool interrupted mid-flight sends no
    // `tool_finished`, so the panel needs the run's own ending to settle it.
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "run_shell", arguments: { command: "sleep 300" } }),
      ev("interrupted", {}),
    ]);
    expect(calls[0].pending).toBe(false);
    expect(calls[0].stopped).toBe(true);
  });

  it("marks it stopped rather than failed", () => {
    // Changing your mind is not an error, and a red step under a stopped turn
    // would say something went wrong.
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "web_fetch", arguments: { url: "https://x" } }),
      ev("interrupted", {}),
    ]);
    expect(calls[0].status).toBeUndefined();
    expect(calls[0].stopped).toBe(true);
  });

  it("leaves calls that did finish exactly as they were", () => {
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "read_file", arguments: { path: "a.md" } }),
      ev("tool_finished", { name: "read_file", status: "ok", result_preview: "contents" }),
      ev("tool_proposed", { name: "run_shell", arguments: { command: "sleep 300" } }),
      ev("interrupted", {}),
    ]);
    expect(calls[0]).toMatchObject({ pending: false, status: "ok", result: "contents" });
    expect(calls[0].stopped).toBeUndefined();
    expect(calls[1].stopped).toBe(true);
  });

  it("settles a step left open by a turn that simply ended", () => {
    // Not every unfinished call is an interruption: a turn can end with a step
    // that never reported. It still must not spin under a finished answer.
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "grep", arguments: { pattern: "TODO" } }),
      ev("turn_done", { artifacts: [] }),
    ]);
    expect(calls[0].pending).toBe(false);
    expect(calls[0].stopped).toBe(true);
  });

  it("a tool started after the stop is still tracked on its own", () => {
    // The backend no longer starts tools after a stop, but a frame already in
    // flight must not be silently folded into the interrupted one.
    const calls = toolCallsFrom([
      ev("tool_proposed", { name: "run_shell", arguments: { command: "a" } }),
      ev("interrupted", {}),
      ev("tool_proposed", { name: "run_shell", arguments: { command: "b" } }),
      ev("tool_finished", { name: "run_shell", status: "ok", result_preview: "done" }),
    ]);
    expect(calls).toHaveLength(2);
    expect(calls[0].stopped).toBe(true);
    expect(calls[1]).toMatchObject({ pending: false, result: "done" });
  });
});
