import { describe, expect, it } from "vitest";
import type { InboxItem } from "./api";
import { isStaleApproval, parkedApproval, primaryArg, requesterLabel, secondaryArgs, unescapeTwice } from "./inboxText";

const item = (over: Partial<InboxItem>): InboxItem => ({
  id: "i1",
  session_id: "s1",
  kind: "approval",
  title: "",
  body: "",
  state: "pending",
  resolution: null,
  inbox: "default",
  created_at: new Date().toISOString(),
  resolved_at: null,
  ...over,
});

describe("unescapeTwice", () => {
  it("undoes the extra round of escaping the parked rows carry", () => {
    expect(unescapeTwice("I\\u2019m looking\\nfor it")).toBe("I’m looking\nfor it");
  });
});

describe("parkedApproval", () => {
  it("reads an MCP call from its title and decodes its body", () => {
    // Verbatim shape from a real Inbox.
    const got = parkedApproval(
      item({
        title: "Run codex on codex",
        body: '{"config":{"approval-policy":"on-request"},"prompt":"In the file app.conf, change the port. I\\\\u2019m sure."}',
      }),
    );
    expect(got?.tool).toBe("codex");
    expect(got?.server).toBe("codex");
    expect(got?.args.prompt).toBe("In the file app.conf, change the port. I’m sure.");
  });

  it("reads a shell command row as run_shell", () => {
    const got = parkedApproval(item({ title: 'Run: find . -name "brief.md"', body: '{"command":"find . -name \\"brief.md\\""}' }));
    expect(got?.tool).toBe("run_shell");
    expect(got?.args.command).toBe('find . -name "brief.md"');
  });

  it("reads the oldest shape: the tool in the title, its argument as a plain body", () => {
    expect(parkedApproval(item({ title: "Approve: run_shell", body: "rm -rf build/" }))).toEqual({
      tool: "run_shell",
      args: { command: "rm -rf build/" },
    });
  });

  it("prefers the structured data when the row has it", () => {
    const got = parkedApproval(item({ data: { tool: "write_file", arguments: { path: "a.md" } } }));
    expect(got).toEqual({ tool: "write_file", args: { path: "a.md" } });
  });

  it("is null for things that are not approvals", () => {
    expect(parkedApproval(item({ kind: "question", title: "What should the brief cover?" }))).toBeNull();
  });
});

describe("primaryArg / secondaryArgs", () => {
  it("shows the request itself and keeps the settings as short lines", () => {
    const args = { config: { "approval-policy": "on-request" }, prompt: "Change the port." };
    expect(primaryArg(args)).toEqual({ key: "prompt", text: "Change the port.", mono: false });
    expect(secondaryArgs(args, "prompt")).toEqual(["approval-policy: on-request"]);
  });
});

describe("requesterLabel", () => {
  it("names automation runs instead of printing their ids", () => {
    expect(requesterLabel(item({ session_title: "__run__run-ed914dd092" }))).toBe("An automation run");
    expect(requesterLabel(item({ data: { task_title: "Morning brief" } }))).toBe("Morning brief (automation)");
    expect(requesterLabel(item({ session_title: "Weekly plan" }))).toBe("Weekly plan");
  });
});

describe("isStaleApproval", () => {
  it("reads the epoch seconds the core actually sends", () => {
    const twoDaysAgo = Date.now() / 1000 - 2 * 24 * 3600;
    expect(isStaleApproval(item({ created_at: twoDaysAgo as unknown as string }))).toBe(true);
  });

  it("is true only for approvals waiting more than a day", () => {
    const old = new Date(Date.now() - 2 * 24 * 3600 * 1000).toISOString();
    expect(isStaleApproval(item({ created_at: old }))).toBe(true);
    expect(isStaleApproval(item({}))).toBe(false);
    expect(isStaleApproval(item({ kind: "question", created_at: old }))).toBe(false);
  });
});
