import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { ApprovalCard } from "./ApprovalCard";
import { humanizeApprovalTitle, humanizeTool } from "../humanize";

// Writing a skill or an assistant from inside a conversation. The card
// is the gate: what it shows is what the person is agreeing to, and it says where the thing
// lands. Neither proposal gets a session-wide "always" — every one is its own decision.

afterEach(cleanup);

const PERSONA = {
  kind: "approval" as const,
  name: "save_persona",
  args: {
    id: "sql-reviewer",
    name: "SQL Reviewer",
    tagline: "Reads migrations before they run",
    instructions: "You review SQL migrations before anyone runs them. Say what each locks and for how long.",
    tools: ["files", "search"],
    mode: "interactive",
  },
  reason: "",
};

describe("a proposed assistant", () => {
  it("shows what it is for, what it may use, and where it lands", () => {
    render(<ApprovalCard item={PERSONA} onApprove={vi.fn()} />);
    expect(screen.getByText("Reads migrations before they run")).toBeTruthy();
    expect(document.body.textContent).toContain("Can use: files, search");
    expect(document.body.textContent).toContain("Acts: interactive");
    expect(document.body.textContent).toContain("adds it to your assistants");
    expect(screen.getByText("Add this assistant")).toBeTruthy();
  });

  it("is never allowed session-wide — one proposal, one decision", () => {
    render(<ApprovalCard item={PERSONA} onApprove={vi.fn()} />);
    expect(screen.queryByText("Always allow")).toBeNull();
  });

  it("reads as adding, not installing", () => {
    expect(humanizeApprovalTitle("save_persona", PERSONA.args)).toEqual({
      pre: "Add the assistant ",
      obj: "SQL Reviewer",
      post: " to your assistants",
    });
    expect(humanizeTool("save_skill", { name: "weekly-ops-review" })).toEqual({
      pre: "Proposed the skill ",
      obj: "weekly-ops-review",
      post: " for you to approve",
    });
    expect(humanizeTool("list_personas", {}).pre).toContain("assistants on this machine");
  });
});
