import { describe, expect, it } from "vitest";
import { format, parse } from "./route";
import { NAV } from "./nav";

describe("route parsing", () => {
  it("maps each surface to a stable address and back", () => {
    for (const path of ["/brain", "/automations", "/inbox", "/connectors", "/activity", "/settings"]) {
      const r = parse(`#${path}`);
      expect(format(r)).toBe(`#${path}`);
    }
  });

  it("gives every nav entry somewhere to point", () => {
    // The nav list and the route table are separate; this is what stops one of
    // them growing an entry the other has never heard of.
    for (const entry of NAV) {
      const r = parse(format({ surface: entry.id }));
      expect(r.surface).toBe(entry.id);
    }
  });

  it("reads a sub-tab out of the query", () => {
    expect(parse("#/brain?tab=timeline")).toEqual({ surface: "brain", tab: "timeline" });
    expect(parse("#/automations?tab=watches")).toEqual({ surface: "scheduled", tab: "watches" });
    expect(format({ surface: "settings", tab: "models" })).toBe("#/settings?tab=models");
  });

  it("addresses a persona by id, round-tripping one that needs escaping", () => {
    expect(parse("#/persona/ops")).toEqual({ surface: "persona", id: "ops" });
    const odd = { surface: "persona" as const, id: "a b/c" };
    expect(parse(format(odd))).toEqual(odd);
  });

  it("addresses a conversation, which was the one thing Step 6 left out", () => {
    expect(parse("#/chat/s-abc123")).toEqual({ surface: "session", id: "s-abc123" });
    expect(format({ surface: "session", id: "s-abc123" })).toBe("#/chat/s-abc123");
    // Round-trips an id that needs escaping — automation runs use `__run__…`.
    const odd = { surface: "session" as const, id: "__run__a b" };
    expect(parse(format(odd))).toEqual(odd);
    // Without an id it's still the session view, which is what `#/` means.
    expect(format({ surface: "session" })).toBe("#/");
    expect(parse("#/chat").surface).toBe("session");
    expect(parse("#/chat").id).toBeUndefined();
  });

  it("keeps renamed and mistyped links working instead of dropping them", () => {
    // `scheduled` is the internal key, `automations` the label — both are things
    // someone will have a link to.
    expect(parse("#/scheduled").surface).toBe("scheduled");
    expect(parse("#/memories").surface).toBe("brain");
    expect(parse("#/connections").surface).toBe("integrations");
    // Trailing slashes are the same place.
    expect(parse("#/brain/").surface).toBe("brain");
  });

  it("falls back to the conversation rather than a blank screen", () => {
    expect(parse("").surface).toBe("session");
    expect(parse("#").surface).toBe("session");
    expect(parse("#/").surface).toBe("session");
    // A surface that no longer exists at all.
    expect(parse("#/tinyplace").surface).toBe("session");
  });
});
