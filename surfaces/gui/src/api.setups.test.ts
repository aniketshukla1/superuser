import { afterEach, describe, expect, it, vi } from "vitest";
import { getBugRepro, getChief, getEmailBot, getExpenses, getPerf } from "./api";

// Each assistant's Settings section reads its setup through one of these getters. A body of
// the wrong shape used to reach the section and throw in its render, which took the whole
// Settings page down: the e2e mock answers unknown routes with `{}`, and 25 specs that only
// wanted to open a Settings tab failed on it.

function answer(json: unknown, ok = true) {
  vi.stubGlobal("fetch", vi.fn(async () => ({ ok, json: async () => json }) as Response));
}

afterEach(() => vi.unstubAllGlobals());

const GETTERS = Object.entries({ getChief, getPerf, getBugRepro, getExpenses, getEmailBot });

describe("assistant setups", () => {
  it.each(GETTERS)("%s: an empty body is no setup, not a broken one", async (_, get) => {
    answer({});
    expect(await get()).toBeNull();
  });

  it.each(GETTERS)("%s: an error status is no setup", async (_, get) => {
    answer({ error: "Not found" }, false);
    expect(await get()).toBeNull();
  });

  it("a real body passes through, with a missing list filled in", async () => {
    answer({ priorities: "Ship 2.41", routine: true });
    expect(await getChief()).toEqual({ priorities: "Ship 2.41", routine: true, sources: [] });

    answer({ connected: true, account: null, signals: "p95: q", count: 1, routine: false });
    expect((await getPerf())?.problems).toEqual([]);
  });
});
