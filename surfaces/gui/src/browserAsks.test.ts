import { describe, expect, it } from "vitest";
import { endBrowserAsks, resolveHelp, resolveSecret } from "./browserAsks";
import type { Item } from "./types";

const help = (reason: string, resolved?: "taken" | "declined" | "ended"): Item => ({
  kind: "computer_help",
  persona: "assistant",
  reason,
  url: "https://example.com/",
  ...(resolved ? { resolved } : {}),
});

describe("requests for the user's hands and values", () => {
  it("settles the newest open request, and only one of its kind", () => {
    const items: Item[] = [
      help("first", "declined"),
      { kind: "computer_secret", persona: "assistant", label: "Password" },
      help("second"),
    ];
    const out = resolveHelp(items, "taken");
    expect(out[2]).toMatchObject({ reason: "second", resolved: "taken" });
    expect(out[0]).toMatchObject({ resolved: "declined" });
    expect(out[1]).not.toHaveProperty("resolved");
    expect(resolveSecret(out, "sent")[1]).toMatchObject({ resolved: "sent" });
  });

  it("leaves the list alone when nothing is open", () => {
    const items: Item[] = [help("done", "taken")];
    expect(resolveHelp(items, "declined")).toBe(items);
    expect(endBrowserAsks(items)).toBe(items);
  });

  it("a finished turn is waiting on nobody", () => {
    const items: Item[] = [help("sign in"), { kind: "computer_secret", persona: "assistant", label: "Code" }, help("old", "taken")];
    const out = endBrowserAsks(items);
    expect(out.map((it) => ("resolved" in it ? it.resolved : undefined))).toEqual(["ended", "ended", "taken"]);
  });
});
