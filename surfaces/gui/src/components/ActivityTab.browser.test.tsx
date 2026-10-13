// What the Browser tab watches for. The selector is half of a cross-language
// coupling — the other half is the Rust tool name, pinned by
// `browser_open_is_the_name_the_panel_watches_for` in core/agent.rs.
import { describe, expect, it } from "vitest";
import {
  BROWSER_OPEN_TOOL,
  MANUAL_TAB_GRACE_MS,
  browserUrlWantedBy,
  shouldSurfaceBrowser,
  type ToolCall,
} from "./ActivityTab";

const call = (over: Partial<ToolCall>): ToolCall => ({
  seq: 1,
  at: 0,
  name: BROWSER_OPEN_TOOL,
  title: "",
  args: {},
  pending: false,
  ...over,
});

describe("browserUrlWantedBy", () => {
  it("is empty when the agent never asked to show anything", () => {
    expect(browserUrlWantedBy([])).toBe("");
    expect(browserUrlWantedBy([call({ name: "web_fetch", args: { url: "https://a.test" } })])).toBe("");
  });

  it("takes the most recent request, not the first", () => {
    expect(
      browserUrlWantedBy([
        call({ seq: 1, args: { url: "https://first.test" } }),
        call({ seq: 2, args: { url: "https://latest.test" } }),
      ]),
    ).toBe("https://latest.test");
  });

  it("ignores a call the server refused", () => {
    // A blocked scheme or a loopback host must not still yank the panel open onto
    // a page that was never loaded.
    expect(
      browserUrlWantedBy([
        call({ seq: 1, args: { url: "https://loaded.test" } }),
        call({ seq: 2, args: { url: "file:///etc/passwd" }, result: "error: that scheme can't be opened here (http/https only)." }),
      ]),
    ).toBe("https://loaded.test");
  });

  it("ignores a call with no url", () => {
    expect(browserUrlWantedBy([call({ args: {} })])).toBe("");
  });
});

describe("when the Browser tab may take over the panel", () => {
  const base = { alreadyWatching: false, typing: false, msSinceManualChoice: 60_000 };

  it("surfaces when the user is idle on another tab", () => {
    expect(shouldSurfaceBrowser(base)).toBe(true);
  });

  it("does nothing when they are already watching", () => {
    expect(shouldSurfaceBrowser({ ...base, alreadyWatching: true })).toBe(false);
  });

  it("never takes the keyboard while they are typing", () => {
    // The whole point of the rule: interrupting mid-sentence is worse than a tab
    // that never opens.
    expect(shouldSurfaceBrowser({ ...base, typing: true })).toBe(false);
  });

  it("does not fight a tab they just chose by hand", () => {
    expect(shouldSurfaceBrowser({ ...base, msSinceManualChoice: 0 })).toBe(false);
    expect(shouldSurfaceBrowser({ ...base, msSinceManualChoice: MANUAL_TAB_GRACE_MS - 1 })).toBe(false);
  });

  it("stops deferring once that choice is old", () => {
    expect(shouldSurfaceBrowser({ ...base, msSinceManualChoice: MANUAL_TAB_GRACE_MS })).toBe(true);
  });
});
