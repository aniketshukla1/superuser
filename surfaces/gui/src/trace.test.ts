import { afterEach, describe, expect, it, vi } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { chars, topicEnabled, trace } from "./trace";

afterEach(() => vi.restoreAllMocks());

describe("dev-only tracing", () => {
  it("filters by topic, and treats no setting as all topics", () => {
    expect(topicEnabled("chat", null)).toBe(true);
    expect(topicEnabled("chat", new Set(["chat"]))).toBe(true);
    expect(topicEnabled("tools", new Set(["chat"]))).toBe(false);
    // "off" parses to an empty set — an absent filter and a deliberately empty one
    // are different answers, and only one of them means "everything".
    expect(topicEnabled("chat", new Set())).toBe(false);
    expect(topicEnabled("CHAT", new Set(["chat"]))).toBe(true);
  });

  it("reports a length, never the words", () => {
    // The rule the whole module is built around: a trace line is the kind of thing
    // that ends up pasted into a bug report, and this app's claim is that
    // conversations stay yours.
    const secret = "my bank password is hunter2";
    expect(chars(secret)).toBe("27 chars");
    expect(chars(secret)).not.toContain("hunter2");
    expect(chars(null)).toBe("0 chars");
    expect(chars(undefined)).toBe("0 chars");
  });

  it("survives a storage that throws instead of taking tracing down with it", () => {
    // A private window can throw on `localStorage` access. Tracing must never be the
    // reason something fails — an unreadable setting has to read as "no filter".
    const spy = vi.spyOn(console, "debug").mockImplementation(() => {});
    vi.stubGlobal("localStorage", {
      getItem() {
        throw new Error("access denied");
      },
    });
    expect(() => trace("chat", "still fine", { a: 1 })).not.toThrow();
    vi.unstubAllGlobals();
    spy.mockRestore();
  });
});

describe("the call-site rule", () => {
  // Every trace call begins `TRACE &&`, and this is why: without the guard the call
  // SURVIVES a production build. The function minifies to `()=>{}` so nothing prints,
  // but the topic, the message and the data object stay in the shipped bundle and are
  // still constructed at runtime for a function that throws them away.
  //
  // That was measured, not assumed. Marking the function pure cleared most call sites
  // and left behind the ones sitting inside comma sequences, where the minifier will
  // not take the risk. `TRACE` is `import.meta.env.DEV`, which Vite replaces with the
  // literal `false`, so the guarded form folds to nothing every time.
  //
  // A rule nobody can forget beats one everybody has to remember.
  const SRC = join(__dirname);

  function sources(dir: string, out: string[] = []): string[] {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) sources(p, out);
      else if (/\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name) && name !== "trace.ts") out.push(p);
    }
    return out;
  }

  it("guards every trace call so production drops it whole", () => {
    const unguarded: string[] = [];
    for (const file of sources(SRC)) {
      const src = readFileSync(file, "utf8");
      for (const line of src.split("\n")) {
        // The call, not the word: `// trace(` in a comment and `sessionTrace(` are
        // not what this is about.
        if (!/(^|[^\w.])trace\(/.test(line)) continue;
        if (line.trimStart().startsWith("*") || line.trimStart().startsWith("//")) continue;
        if (!line.includes("TRACE && trace(")) unguarded.push(`${file.split("/src/")[1]}: ${line.trim()}`);
      }
    }
    expect(unguarded, "these would ship their message strings in the production bundle").toEqual([]);
  });
});
