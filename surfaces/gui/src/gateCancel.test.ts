import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { fallbackAgent, gateCancelTarget, type GateReturn } from "./gateCancel";
import { readModule } from "./testing/source";

const gates = (a: string) => ["coder", "security", "cloud-posture", "dep-audit", "lead", "qa", "writer"].includes(a);

const FROM_CODER: GateReturn = {
  agent: "coder",
  workspace: "/Users/me/aegis",
  sessionId: "1b6f0a58",
  branch: "feature/x",
};

describe("cancelling the folder gate", () => {
  it("returns to the exact conversation the gate interrupted", () => {
    // The bug: picking Security from the New menu switched the agent and cleared the
    // workspace, and cancelling then landed on Home with somebody else's chat open.
    expect(gateCancelTarget(FROM_CODER, null, gates)).toEqual({
      kind: "resume",
      agent: "coder",
      workspace: "/Users/me/aegis",
      sessionId: "1b6f0a58",
      branch: "feature/x",
    });
  });

  it("returns to a folderless assistant too, without inventing a folder for it", () => {
    // Coming from Main — no workspace, and none needed. The empty string is what
    // `selectSession` reads as "leave the workspace alone", not as a path.
    const back: GateReturn = { agent: "assistant", workspace: null, sessionId: "s2", branch: null };
    expect(gateCancelTarget(back, null, gates)).toEqual({
      kind: "resume",
      agent: "assistant",
      workspace: "",
      sessionId: "s2",
      branch: null,
    });
  });

  it("refuses to return to a workbench that has no folder", () => {
    // This is the trap. That state is precisely what the gate exists to prevent, so
    // "returning" to it hands back the dead screen instead of undoing the navigation.
    const back: GateReturn = { agent: "coder", workspace: null, sessionId: "s3", branch: null };
    expect(gateCancelTarget(back, null, gates)).toEqual({ kind: "fallback" });
  });

  it("ignores a return point with no conversation in it", () => {
    const back = { agent: "coder", workspace: "/Users/me/aegis", sessionId: "", branch: null };
    expect(gateCancelTarget(back, null, gates)).toEqual({ kind: "fallback" });
  });

  it("just closes when the gate was raised over a working screen", () => {
    // Change-folder from inside a project: there is no navigation to undo, and moving
    // the user anywhere would be the app answering a question it was not asked.
    expect(gateCancelTarget(null, "/Users/me/aegis", gates)).toEqual({ kind: "stay" });
  });

  it("falls back to an assistant only on a true first run", () => {
    expect(gateCancelTarget(null, null, gates)).toEqual({ kind: "fallback" });
  });
});

describe("the fallback assistant", () => {
  const personas = [
    { id: "coder", kind: "workbench" },
    { id: "security", kind: "workbench" },
    { id: "assistant", kind: "assistant", default: true },
    { id: "support", kind: "assistant" },
  ];

  it("prefers the default, then any assistant, then the general id", () => {
    expect(fallbackAgent(personas, "assistant")).toBe("assistant");
    expect(fallbackAgent(personas.filter((p) => p.id !== "assistant"), "assistant")).toBe("support");
    expect(fallbackAgent(null, "assistant")).toBe("assistant");
    expect(fallbackAgent([], "assistant")).toBe("assistant");
  });

  it("is never a workbench", () => {
    // A workbench needs a folder — the question just declined. Landing on one reopens
    // the gate the user was trying to leave.
    const onlyWorkbenches = personas.filter((p) => p.kind === "workbench");
    expect(fallbackAgent(onlyWorkbenches, "assistant")).toBe("assistant");
    // Even when a workbench claims to be the default.
    expect(fallbackAgent([{ id: "coder", kind: "workbench", default: true }], "assistant")).toBe("assistant");
  });
});

describe("every gate has a return point", () => {
  // The fix is only worth as much as its coverage. Two of the four places that raise
  // the gate had it; `switchAgent`'s two did not, so reaching the same dialog from a
  // persona header still dumped you on Home. A gate raised without recording what it
  // navigated away from cannot be cancelled correctly, and nothing at the call site
  // says so — hence this.
  const src = readModule(join(__dirname, "App.tsx"));
  /** App.tsx with `openGate`'s own definition cut out — it is the one place allowed to
   *  raise the gate, so leaving it in would make the rule pass by describing itself. */
  const callers = src.replace(/const openGate = \([\s\S]*?\n  \};\n/, "");

  it("raises the gate only through openGate", () => {
    const opens = [...callers.matchAll(/setShowGate\(([^)]*)\)/g)]
      .map((m) => m[1].trim())
      .filter((arg) => arg !== "false");
    expect(opens).toEqual([]);
  });

  it("passes openGate something to go back to, or an explicit nothing", () => {
    const calls = [...callers.matchAll(/openGate\(([^;]*?)\);/g)].map((m) => m[1].trim());
    // Five sites: boot, startNewSession, switchAgent ×2, newProject.
    expect(calls.length).toBeGreaterThanOrEqual(5);
    for (const arg of calls) {
      // `null` is a real answer — boot reaches the gate because there was nothing to
      // resume. Anything else has to carry a conversation, whether written inline or
      // captured in a local first, or cancel has nothing to undo and silently takes
      // the fallback path forever.
      const declared = src.match(new RegExp(`const ${arg}[^=]*= (\\{[^}]*\\})`))?.[1] ?? "";
      expect(arg === "null" || /sessionId/.test(arg) || /sessionId/.test(declared)).toBe(true);
    }
  });
});
