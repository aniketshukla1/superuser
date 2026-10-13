import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { readModule } from "./testing/source";

// An effect that WATCHES the active agent and also SETS it can undo a choice the
// user just made, one render later, with nothing on screen connecting the two.
//
// That is not hypothetical. A guard here watched `[agent, surfaces]` and called
// `switchAgent` when the agent's "surface" was hidden — a concept the persona
// roster replaced, using a flag that defaults to false. Clicking a Coder session in
// the Workspace list called `selectSession(thatSession, aegis, "coder")`, the effect
// fired on the very next render, and you landed on the general assistant looking at
// a different conversation entirely. The click appeared to open the wrong chat.
//
// `selectSession` is the user speaking. Nothing may overrule it from an effect.

const APP = join(__dirname, "App.tsx");

/** Every `useEffect(...)` body in a source file, with its dependency array. */
function effects(src: string): { body: string; deps: string }[] {
  const out: { body: string; deps: string }[] = [];
  const start = /useEffect\(\s*\(\s*\)\s*=>\s*\{/g;
  for (let m = start.exec(src); m; m = start.exec(src)) {
    // Walk braces from the opening `{` to find the body, then read the deps that
    // follow it. A regex alone cannot balance braces, and getting this wrong would
    // make the test lie in the reassuring direction.
    let depth = 1;
    let i = m.index + m[0].length;
    for (; i < src.length && depth > 0; i++) {
      if (src[i] === "{") depth++;
      else if (src[i] === "}") depth--;
    }
    const body = src.slice(m.index + m[0].length, i - 1);
    const tail = src.slice(i, i + 200);
    out.push({ body, deps: tail.match(/^\s*,\s*(\[[^\]]*\])/)?.[1] ?? "" });
  }
  return out;
}

describe("App effects", () => {
  it("no effect that watches the agent also changes it", () => {
    const src = readModule(APP);
    const all = effects(src);
    expect(all.length, "the reader found no effects — its pattern has drifted").toBeGreaterThan(5);

    const offenders = all
      .filter((e) => /\bagent\b/.test(e.deps))
      .filter((e) => /\b(switchAgent|setAgent)\s*\(/.test(e.body.replace(/\/\/[^\n]*/g, "")))
      .map((e) => e.body.trim().split("\n")[0].slice(0, 80));

    expect(
      offenders,
      `these effects watch the agent and set it, so they can overrule a session the user just clicked:\n  ${offenders.join(
        "\n  ",
      )}`,
    ).toEqual([]);
  });

  it("the hidden-surface flags steer nothing, now that personas carry availability", () => {
    // `surface_chat` / `surface_code` predate the roster. While anything switched on
    // them, an assistant could be yanked away from you by a setting that no screen
    // in this app still writes.
    const src = readModule(APP).replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
    const steering = [...src.matchAll(/surfaces\.(chat|code|cowork)/g)].map((m) => m[0]);
    expect(steering, `App.tsx still branches on ${steering.join(", ")}`).toEqual([]);
  });
});

describe("switching conversations", () => {
  const src = readModule(APP);

  it("clears the previous turn on BOTH paths into a conversation", () => {
    // There are two ways to land in a conversation: clicking a row in the sidebar
    // (`selectSession`) and the URL changing (Back, Forward, a pasted link). The
    // second one's comment claimed it switched "exactly as clicking the row in the
    // sidebar would" — and it did not. It adopted the id and loaded the transcript,
    // leaving the live turn of the chat you came FROM on screen: the status line kept
    // ticking and the composer stayed locked on "Working…" in a conversation where
    // nothing was running, and no event was coming to correct it.
    //
    // Two paths described as equivalent, one quietly doing less. The fix is that both
    // call one function, so this asserts they both still do.
    // Matches the call however it is spelled — it grew a `via` argument for the
    // trace, and a test that pins the argument list fails on changes that are not
    // the one it exists to catch.
    const calls = (src.match(/leaveConversation\(["'\w]/g) || []).length;
    expect(calls).toBeGreaterThanOrEqual(2);

    // And that the reset actually clears what a running turn puts on screen. A
    // conversation that IS running gets these back from the server's replayed
    // `turn_start`; the point is that a quiet one does not.
    const body = src.slice(src.indexOf("const leaveConversation ="));
    const fn = body.slice(0, body.indexOf("};"));
    for (const setter of ["setRunning(false)", "setStreaming(\"\")", "setTurnTokens(0)"]) {
      expect(fn).toContain(setter);
    }
  });
});
