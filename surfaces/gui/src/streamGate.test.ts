import { describe, expect, it } from "vitest";
import { streamMode, waitingLabel, STREAM_PROMOTE_WORDS } from "./streamGate";
import type { Item } from "./types";

const USER: Item[] = [{ kind: "user", text: "build the dashboard" }];
const AFTER_TOOL: Item[] = [
  ...USER,
  { kind: "tool", id: "t1", name: "read_file", args: { path: "a.md" }, status: "ok" },
];

const words = (n: number) => Array.from({ length: n }, (_, i) => `w${i}`).join(" ");

describe("stream gate (§33 refinement #3 — one rule for all streamed text)", () => {
  it("turn-start under the threshold is HELD (spinner, nothing rendered)", () => {
    expect(streamMode("Checking what merged since yesterday.", USER, true)).toBe("hold");
    expect(streamMode(words(STREAM_PROMOTE_WORDS - 1), USER, true)).toBe("hold");
  });

  it("mid-turn under the threshold is QUIET — it belongs to the live turn group", () => {
    // The owner's exact repro: ~25 words of mid-turn narration must never float.
    expect(
      streamMode(
        "The quote endpoint rate-limited, so I'm checking whether the historical pages expose older pages for January closes.",
        AFTER_TOOL,
        true,
      ),
    ).toBe("quiet");
  });

  it("crossing the threshold promotes to the ANSWER bubble — start and mid-turn alike", () => {
    expect(streamMode(words(STREAM_PROMOTE_WORDS), USER, true)).toBe("answer");
    expect(streamMode(words(STREAM_PROMOTE_WORDS), AFTER_TOOL, true)).toBe("answer");
  });

  it("no text → none; not running → answer (never swallow text on a settling session)", () => {
    expect(streamMode("", USER, true)).toBe("none");
    expect(streamMode("short", USER, false)).toBe("answer");
  });

  it("notices are transparent to the turn-position check", () => {
    const items: Item[] = [...AFTER_TOOL, { kind: "notice", tone: "info", text: "fyi" }];
    expect(streamMode("checking…", items, true)).toBe("quiet");
  });
});

describe("waiting row label", () => {
  it("says 'Waiting for agent…' only when the turn really hasn't started", () => {
    expect(waitingLabel(USER)).toBe("Waiting for agent…");
    expect(waitingLabel([])).toBe("Waiting for agent…");
  });

  it("yields while a step is in flight — the step group already shows that state", () => {
    const running: Item[] = [
      ...USER,
      { kind: "tool", id: "t1", name: "write_file", args: { path: "a.txt" }, status: "…" },
    ];
    expect(waitingLabel(running)).toBeNull();
  });

  it("says 'Thinking…' once steps have settled and the model owes the next move", () => {
    // The reported contradiction: a completed step above, "Waiting for agent…" below.
    expect(waitingLabel(AFTER_TOOL)).toBe("Thinking…");
    // A failed step is still a step that ran.
    expect(
      waitingLabel([...USER, { kind: "tool", id: "t1", name: "grep", args: {}, status: "error" }]),
    ).toBe("Thinking…");
  });

  it("ignores notices, which say nothing about what we're waiting for", () => {
    const withNotice: Item[] = [...AFTER_TOOL, { kind: "notice", tone: "info", text: "Context compacted" }];
    expect(waitingLabel(withNotice)).toBe("Thinking…");
  });
});

describe("waiting behind another chat", () => {
  // A local runner serves a fixed number of sequences at once (Ollama starts its
  // backend with `-np N`, 1 by default), so a second chat on the same model really is
  // queued. Both chats used to sit on "Waiting for agent…" for as long as the other
  // took, with nothing to say the app was fine and the model was busy.
  const justAsked: Item[] = [{ kind: "user", text: "search the web for X" }];

  it("says so when nothing has happened in this turn yet", () => {
    expect(waitingLabel(justAsked, true)).toBe("Waiting its turn — another chat is using this model");
    expect(waitingLabel([], true)).toBe("Waiting its turn — another chat is using this model");
  });

  it("keeps the old wording when there is no contention", () => {
    expect(waitingLabel(justAsked, false)).toBe("Waiting for agent…");
    expect(waitingLabel(justAsked)).toBe("Waiting for agent…");
  });

  it("stops explaining the queue once this turn has done something", () => {
    // Its own work is what it is waiting on now — the queue is no longer the story,
    // and a row that keeps blaming another chat would be describing the wrong thing.
    const afterAStep: Item[] = [
      { kind: "user", text: "search" },
      { kind: "tool", id: "t1", name: "web_search", args: {}, status: "ok" },
    ];
    expect(waitingLabel(afterAStep, true)).toBe("Thinking…");
    const midNarration: Item[] = [
      { kind: "user", text: "search" },
      { kind: "assistant", text: "Looking that up." },
    ];
    expect(waitingLabel(midNarration, true)).toBe("Thinking…");
  });

  it("still yields to a tool in flight and to a pending approval", () => {
    // Contention must not resurrect a row these two deliberately silence.
    const running: Item[] = [{ kind: "tool", id: "t1", name: "web_search", args: {}, status: "…" }];
    expect(waitingLabel(running, true)).toBeNull();
    const asking: Item[] = [{ kind: "approval", id: "a1", name: "run_shell", args: {} } as never];
    expect(waitingLabel(asking, true)).toBeNull();
  });
});
