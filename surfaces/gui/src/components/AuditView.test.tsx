import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AuditView } from "./AuditView";

// The Actions tab has to say three things a flat list of rows never did: which
// boundary refused an action, that nobody was watching the run that tried it, and
// whether the trail itself has been edited.

const REFUSED = {
  id: 0,
  timestamp: "2026-09-14T10:00:00+00:00",
  session_id: "__run__run-1",
  agent: "assistant",
  workspace: "",
  connector: "",
  tool: "send_message",
  stage: "done",
  status: "refused",
  approval: "",
  args: { target: "slack:#ops" },
  result_preview: "sending a message to slack:#ops — refused",
  at: 1757844000,
  event: "action.refused",
  actor: "assistant",
  target: "slack:#ops",
  detail: "sending a message to slack:#ops — refused",
  ok: false,
  persona: "assistant",
  initiator: { kind: "schedule", id: "run-1" },
  unwatched: true,
  subject: { tool: "send_message", intent: "send", target: "slack:#ops", chars: 42 },
  decision: { allowed: false, mode: "enforce", source: "deny", rule: 'intent == "send"', rule_name: "Never send messages", carried_out: false },
};

function stubFetch(integrity: any = { rows: 3, linked: 3, legacy: 0, intact: true }) {
  const calls: string[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      const u = String(url);
      calls.push(u);
      const reply = (v: any) => ({ ok: true, json: async () => v }) as Response;
      if (u.includes("/v1/audit/verify")) return reply(integrity);
      if (u.includes("/v1/audit")) return reply({ events: [REFUSED] });
      if (u.includes("/v1/settings")) return reply({ audit_retention_days: 0 });
      return reply({ events: [] });
    }),
  );
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("Privacy ▸ Actions", () => {
  it("shows which boundary refused an action and that nobody was watching", async () => {
    stubFetch();
    render(<AuditView />);
    fireEvent.click(screen.getByTestId("privacy-tab-actions"));
    const row = await screen.findByTestId("audit-row");
    expect(row.textContent).toContain("Refused");
    expect(screen.getByTestId("audit-rule").textContent).toContain("Never send messages");
    expect(screen.getByTestId("audit-unwatched")).toBeTruthy();
    expect(row.textContent).toContain("started by a schedule");
  });

  it("asks the server for only unwatched runs when the box is ticked", async () => {
    const calls = stubFetch();
    render(<AuditView />);
    fireEvent.click(screen.getByTestId("privacy-tab-actions"));
    await screen.findByTestId("audit-row");
    fireEvent.click(screen.getByTestId("audit-nobody-watching"));
    await waitFor(() => expect(calls.some((c) => c.includes("nobody_watching=1"))).toBe(true));
    fireEvent.change(screen.getByTestId("audit-show"), { target: { value: "refused" } });
    await waitFor(() => expect(calls.some((c) => c.includes("refused=1") && c.includes("nobody_watching=1"))).toBe(true));
  });

  it("says in words where the trail stops adding up", async () => {
    stubFetch({ rows: 4, linked: 4, legacy: 0, intact: false, problem: "Row 2 of audit.jsonl was changed after it was written." });
    render(<AuditView />);
    fireEvent.click(screen.getByTestId("privacy-tab-actions"));
    expect((await screen.findByTestId("audit-integrity-broken")).textContent).toContain("Row 2 of audit.jsonl was changed");
  });
});
