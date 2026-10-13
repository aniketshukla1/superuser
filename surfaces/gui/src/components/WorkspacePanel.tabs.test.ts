import { describe, expect, it } from "vitest";
import { panelTabs, toolPress } from "./WorkspacePanel";

describe("the panel's tab strip", () => {
  it("does not stand a Terminal and a Browser there before anyone asks", () => {
    const ids = panelTabs(null).map((t) => t.id);
    expect(ids).toEqual(["activity", "assistant", "files", "access"]);
  });

  it("adds the tool the topbar opened, and only that one", () => {
    expect(panelTabs("terminal").map((t) => t.id)).toEqual([
      "activity",
      "assistant",
      "files",
      "access",
      "terminal",
    ]);
    expect(panelTabs("browser").map((t) => t.id)).toEqual([
      "activity",
      "assistant",
      "files",
      "access",
      "browser",
    ]);
  });

  // The tool joins the END of the strip. Inserting it among the fixed tabs would move the
  // ones you were aiming at, every time a tool opened.
  it("appends rather than reordering what was already there", () => {
    const before = panelTabs(null).map((t) => t.id);
    const after = panelTabs("terminal").map((t) => t.id);
    expect(after.slice(0, before.length)).toEqual(before);
  });
});

// The whole point of the change: asking for one tool is not a statement about the other.
describe("two tools at once", () => {
  it("shows both when both are open", () => {
    expect(panelTabs(["terminal", "browser"]).map((t) => t.id)).toEqual([
      "activity", "assistant", "files", "access", "terminal", "browser",
    ]);
  });

  // Opened browser-first, the strip must still read terminal-then-browser. A strip that
  // reorders itself by recency moves the target out from under the cursor you are about
  // to click with.
  it("orders by the strip, not by when you opened them", () => {
    expect(panelTabs(["browser", "terminal"]).map((t) => t.id)).toEqual(
      panelTabs(["terminal", "browser"]).map((t) => t.id),
    );
  });

  it("still handles one, and none", () => {
    expect(panelTabs(["browser"]).map((t) => t.id)).toEqual([
      "activity", "assistant", "files", "access", "browser",
    ]);
    expect(panelTabs([]).map((t) => t.id)).toEqual(["activity", "assistant", "files", "access"]);
  });

  // The signature used to take a single id. A caller that has not been updated should
  // get a sensible strip rather than an empty one.
  it("tolerates the old single-value shape", () => {
    expect(panelTabs("terminal").map((t) => t.id)).toContain("terminal");
    expect(panelTabs(null).map((t) => t.id)).toEqual(["activity", "assistant", "files", "access"]);
  });
});

// The strip went icon-only to match the left sidebar's Home/Workspace control. Every
// tab therefore has to carry its name somewhere a person and a screen reader can both
// reach — an unlabelled icon is a guess, and four of them are four guesses.
describe("the icon-only strip", () => {
  it("every tab still has a name to put in aria-label and title", () => {
    for (const t of panelTabs("terminal")) {
      expect(t.label, `${t.id} has no label`).toBeTruthy();
      expect(t.icon, `${t.id} has no icon`).toBeTruthy();
    }
  });

  // Two tabs sharing an icon would be indistinguishable once the words are gone.
  it("no two tabs share an icon", () => {
    const icons = panelTabs("terminal").map((t) => t.icon);
    expect(new Set(icons).size).toBe(icons.length);
  });
});

// The topbar button is a toggle now. Which of the three things it does is the part worth
// pinning down, because two of them look identical from the outside until you press.
describe("pressing a topbar tool button", () => {
  it("opens a tool that is not open, and goes there", () => {
    expect(toolPress("terminal", "files", [], "files")).toEqual({
      open: ["terminal"],
      tab: "terminal",
    });
  });

  // The important one. Pressing "Terminal" while looking at Files means "show me the
  // terminal" — closing something you cannot currently see would be a trap.
  it("focuses a tool that is open but not showing, and closes nothing", () => {
    expect(toolPress("terminal", "browser", ["terminal", "browser"], "files")).toEqual({
      open: ["terminal", "browser"],
      tab: "terminal",
    });
  });

  it("puts away the tool you are looking at, and lands where you were", () => {
    expect(toolPress("terminal", "terminal", ["terminal", "browser"], "files")).toEqual({
      open: ["browser"],
      tab: "files",
    });
  });

  it("leaves the other tool alone when one is put away", () => {
    const after = toolPress("browser", "browser", ["terminal", "browser"], "activity");
    expect(after.open).toEqual(["terminal"]);
  });

  // Press, press: back exactly where you started.
  it("round-trips", () => {
    const opened = toolPress("browser", "files", [], "files");
    const closed = toolPress("browser", opened.tab, opened.open, "files");
    expect(closed).toEqual({ open: [], tab: "files" });
  });

  it("does not mutate the list it was given", () => {
    const open: ("terminal" | "browser")[] = ["terminal"];
    toolPress("browser", "files", open, "files");
    toolPress("terminal", "terminal", open, "files");
    expect(open).toEqual(["terminal"]);
  });
});
