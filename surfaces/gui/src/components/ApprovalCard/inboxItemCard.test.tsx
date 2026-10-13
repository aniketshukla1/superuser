import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ApprovalCard } from "../ApprovalCard";
import { InboxItemCard } from "../InboxItemCard";
import type { Item } from "../../types";
import type { InboxItem } from "../../api";

type ApprovalItem = Extract<Item, { kind: "approval" }>;

const sendApproval = (extra: Partial<ApprovalItem> = {}): ApprovalItem => ({
  kind: "approval",
  name: "send_message",
  args: { target: "slack:T1/C1", text: "digest" },
  reason: "requires approval",
  category: "messaging",
  ...extra,
});

afterEach(cleanup);
describe("InboxItemCard — Allow every time on parked run approvals", () => {
  const baseItem = (data?: Record<string, any>): InboxItem => ({
    id: "i1",
    session_id: "__run__r1",
    kind: "approval",
    title: "Run `send_message`?",
    body: "target: slack:T1/C1",
    state: "pending",
    resolution: null,
    inbox: "default",
    created_at: "",
    resolved_at: null,
    data,
  });

  it("shows the button only when the item carries the task binding + target", () => {
    const onResolve = vi.fn();
    render(
      <InboxItemCard
        item={baseItem({ task_id: "task-1", task_title: "Weekly digest", standing_target: "slack:T1/C1" })}
        onResolve={onResolve}
      />,
    );
    fireEvent.click(screen.getByText("Allow every time"));
    expect(onResolve).toHaveBeenCalledWith("i1", "always_task");
    cleanup();

    // A plain unattended-session approval (no task data) keeps Allow once/Deny only — the same
    // words every approval uses now, parked or live, with tool data or without.
    render(<InboxItemCard item={baseItem()} onResolve={vi.fn()} />);
    expect(screen.queryByText("Allow every time")).toBeNull();
    expect(screen.getByText("Allow once")).toBeTruthy();
    expect(screen.getByText("Deny")).toBeTruthy();
  });

  it("parked approvals with tool data wear the §35 dress — same dialect as the live card", () => {
    const onResolve = vi.fn();
    render(
      <InboxItemCard
        item={baseItem({
          tool: "write_file",
          arguments: { path: "src/fetch_data.py", content: "import json\nx = 1" },
        })}
        onResolve={onResolve}
      />,
    );
    // Humanized title + preview from the args; the raw "Run `write_file`?" title is gone.
    expect(screen.getByText("fetch_data.py")).toBeTruthy();
    expect(screen.queryByText("Run `send_message`?")).toBeNull();
    expect(screen.getByText(/import json/)).toBeTruthy();
    expect(screen.getByText(/stays on this computer/)).toBeTruthy();
    // §35 labels; resolution vocabulary unchanged (works on every approver path).
    fireEvent.click(screen.getByText("Allow once"));
    expect(onResolve).toHaveBeenCalledWith("i1", "allow");
    // Old rows without tool data keep the legacy treatment (covered above).
  });
});

describe("ApprovalCard — save_skill (SKILLS-SPEC §5.2)", () => {
  const skillApproval = (extra: Partial<ApprovalItem> = {}): ApprovalItem =>
    sendApproval({
      name: "save_skill",
      category: "skills",
      args: {
        name: "weekly-github-report",
        description: "Create a concise Monday status report from GitHub activity.",
        instructions: "1. Fetch PRs\n2. Write the report",
        files: ["fetch_prs.py", "sub/example-report.md"],
      },
      standingTarget: undefined,
      ...extra,
    });

  it("shows name-first title, description, instructions, and every bundled file", () => {
    render(<ApprovalCard item={skillApproval()} onApprove={vi.fn()} />);
    expect(screen.getByText("weekly-github-report")).toBeTruthy(); // bold obj in the title
    expect(screen.getAllByText(/to your skills/).length).toBeGreaterThan(0); // title + footer
    // The corner answers WHERE; the footer answers what approving means (§5.2 review round).
    expect(screen.getByText("saves to Settings ▸ Skills")).toBeTruthy();
    expect(screen.getByText(/usable in every conversation from\s+then on/)).toBeTruthy();
    expect(
      screen.getByText("Create a concise Monday status report from GitHub activity."),
    ).toBeTruthy();
    expect(screen.getByText(/Fetch PRs/)).toBeTruthy();
    const chips = screen.getByTestId("skill-bundle-files");
    expect(chips.textContent).toContain("fetch_prs.py");
    expect(chips.textContent).toContain("example-report.md"); // basename, not the path
  });

  it("uses the §7 button copy and never offers a session-wide always", () => {
    const onApprove = vi.fn();
    render(<ApprovalCard item={skillApproval()} onApprove={onApprove} />);
    expect(screen.queryByText("Always allow")).toBeNull(); // every proposal gets its own review
    expect(screen.queryByText("Deny")).toBeNull();
    fireEvent.click(screen.getByText("Add to my skills"));
    expect(onApprove).toHaveBeenCalledWith("once");
    fireEvent.click(screen.getByText("Not now"));
    expect(onApprove).toHaveBeenCalledWith("deny");
  });
});

describe("InboxItemCard — parked save_skill proposals (SKILLS-SPEC §5.2)", () => {
  const parked = (): InboxItem => ({
    id: "i9",
    session_id: "s1",
    kind: "approval",
    title: "Run `save_skill`?",
    body: "",
    state: "pending",
    resolution: null,
    inbox: "default",
    created_at: "",
    resolved_at: null,
    data: {
      tool: "save_skill",
      arguments: {
        name: "weekly-github-report",
        description: "Create a concise Monday status report from GitHub activity.",
        instructions: "1. Fetch PRs\n2. Write the report",
        files: ["fetch_prs.py"],
      },
    },
  });

  it("wears the same review surface and button copy as the live card", () => {
    const onResolve = vi.fn();
    render(<InboxItemCard item={parked()} onResolve={onResolve} />);
    expect(screen.getByText("saves to Settings ▸ Skills")).toBeTruthy();
    expect(
      screen.getByText("Create a concise Monday status report from GitHub activity."),
    ).toBeTruthy();
    expect(screen.getByText(/Fetch PRs/)).toBeTruthy();
    expect(screen.getByTestId("skill-bundle-files").textContent).toContain("fetch_prs.py");
    expect(screen.getByText(/usable in every conversation/)).toBeTruthy();
    expect(screen.queryByText("Allow once")).toBeNull();
    fireEvent.click(screen.getByText("Add to my skills"));
    expect(onResolve).toHaveBeenCalledWith("i9", "allow");
    fireEvent.click(screen.getByText("Not now"));
    expect(onResolve).toHaveBeenCalledWith("i9", "deny");
  });
});
