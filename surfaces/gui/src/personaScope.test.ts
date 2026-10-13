import { describe, expect, it } from "vitest";
import { agentSharesShell, fullPersonaName, shortPersonaName, showsBrowser, showsTerminal } from "./personaScope";

// The detail page's heading used to be manufactured: anything not ending in
// "Assistant" got " Assistant" appended. That was right when manifests said "Ops"
// and wrong the moment the roster grew job titles — "HR Partner Assistant" renames
// a role nobody asked to rename. These pin which half of that survived.

describe("persona display names", () => {
  it("shows a role's own name, not a manufactured one", () => {
    expect(fullPersonaName("HR Partner", "hr")).toBe("HR Partner");
    expect(fullPersonaName("Finance Analyst", "finance")).toBe("Finance Analyst");
    expect(fullPersonaName("Customer Support", "support")).toBe("Customer Support");
    expect(fullPersonaName("QA Analyst", "qa")).toBe("QA Analyst");
  });

  it("leaves the names that already say Assistant alone", () => {
    expect(fullPersonaName("Security Assistant", "security")).toBe("Security Assistant");
    expect(fullPersonaName("Ops Assistant", "ops")).toBe("Ops Assistant");
  });

  it("still maps the legacy cowork id", () => {
    // Predates the rename; stored state can still carry it.
    expect(fullPersonaName(undefined, "cowork")).toBe("Assistant");
  });

  it("falls back to the id when there is no name", () => {
    expect(fullPersonaName(undefined, "finance")).toBe("finance");
    expect(fullPersonaName(undefined, undefined)).toBe("");
  });

  it("chrome still drops a trailing Assistant, and only that", () => {
    expect(shortPersonaName("Security Assistant", "security")).toBe("Security");
    expect(shortPersonaName("HR Partner", "hr")).toBe("HR Partner");
  });
});

describe("the topbar's tool buttons", () => {
  // Browser is a capability button: the panel it opens can only ever be filled by
  // `browser_open`, which `web` grants. Terminal is not — it is the user's own login
  // shell, and there is always one to give them.
  const coder = { tools: ["shell", "files"] };
  const main = { tools: ["recall_memory", "knowledge", "web", "files", "todo"] };
  const analyst = { tools: ["files", "search", "todo", "knowledge", "recall_memory"] };

  it("gives Browser to a web-capable persona that has no shell", () => {
    // The bug: both buttons hung off `shell`, so Main could call `browser_open`,
    // load a page into the panel, and leave the user with no button to reach it.
    expect(showsBrowser(main)).toBe(true);
  });

  it("withholds Browser from a persona that cannot fill it", () => {
    // Coder has the shell and no `web`: its Browser button opened a tab nothing
    // could ever load a page into.
    expect(showsBrowser(coder)).toBe(false);
    expect(showsBrowser(analyst)).toBe(false);
  });

  it("offers the terminal to everyone, because the terminal is the user's", () => {
    // `terminal_open` takes a directory and nothing else — no persona, no capability
    // check. Gating it on `shell` asked whether the ASSISTANT may run commands and
    // used the answer to decide whether the USER may open a shell.
    for (const p of [coder, main, analyst, {}, undefined, null]) {
      expect(showsTerminal(p)).toBe(true);
    }
  });

  it("still knows whether the agent shares that shell, because the label says so", () => {
    // The capability did not stop mattering — only what it decides did. "The shell
    // this session shares" is a false claim on a persona that cannot run a command.
    expect(agentSharesShell(coder)).toBe(true);
    expect(agentSharesShell(main)).toBe(false);
    expect(agentSharesShell(analyst)).toBe(false);
    expect(agentSharesShell(undefined)).toBe(false);
  });
});
