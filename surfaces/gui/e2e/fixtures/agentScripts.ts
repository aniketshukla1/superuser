export type Send = (type: string, data?: Record<string, unknown>) => void;

/** What the fake agent remembers between messages on one socket. */
export type AgentState = {
  /** which proposal the next approval decision resolves */
  pendingTool: string;
  /** the slow stream, stoppable via interrupt */
  epicTimer: ReturnType<typeof setInterval> | null;
  /** a user_message landed — set_model is now a mid-session switch */
  hadTurn: boolean;
};

/** The scripted turns a spec triggers by phrase. True when the script owns the turn (it is
 * suspended on an approval or finishes on its own); false to fall through to the echo. */
export function scriptedTurn(msg: { text: string }, send: Send, st: AgentState): boolean {
  if (/run a tool/i.test(msg.text)) {
    st.pendingTool = "run_shell";
    send("tool_proposed", { name: "run_shell", arguments: { command: "ls" } });
    send("permission_required", {
      name: "run_shell",
      arguments: { command: "ls" },
      reason: "The coworker wants to run a command.",
    });
    return true; // suspended on the approval
  }
  // §35 compact row: a routine workspace write (content rides in the args).
  if (/write a file/i.test(msg.text)) {
    st.pendingTool = "write_file";
    const args = {
      path: "src/fetch_data.py",
      content: "import json\nimport urllib.request\n\ncompanies = [\"NVDA\", \"AMD\"]\nprint(len(companies))\ndone = True",
    };
    send("tool_proposed", { name: "write_file", arguments: args });
    send("permission_required", { name: "write_file", arguments: args, reason: "" });
    return true; // suspended on the approval
  }
  // A one-paragraph digest with NO newlines — the owner-repro shape that once
  // ballooned the card to full-transcript height (char clamp, 2026-07-15).
  if (/post the long digest/i.test(msg.text)) {
    st.pendingTool = "send_message";
    const args = {
      target: "slack:T1/C1",
      text:
        "aisuite — last 24 hours of work (through Jul 15): 5 PRs merged covering chat-completion streaming with unified chunks across providers, multimodal input conversion, Slack collaboration improvements, human attribution for outbound posts, and repo-wide formatting. ".repeat(
          6,
        ),
    };
    send("tool_proposed", { name: "send_message", arguments: args });
    send("permission_required", { name: "send_message", arguments: args, reason: "", category: "messaging" });
    return true;
  }
  // Standing scoped approvals (§25): an eligible connector-ish write — the event
  // carries the pinnable target, exactly like the real engine computes it.
  if (/post the digest/i.test(msg.text)) {
    st.pendingTool = "send_message";
    send("tool_proposed", {
      name: "send_message",
      arguments: { target: "slack:T1/C1", text: "Weekly digest ready" },
    });
    send("permission_required", {
      name: "send_message",
      arguments: { target: "slack:T1/C1", text: "Weekly digest ready" },
      reason: "",
      category: "messaging",
      standing_target: "slack:T1/C1",
    });
    return true;
  }
  // §25 consent card: the agent proposes the automation's permission set on the
  // gated create call; the existing approval card renders disclosure/grant lines.
  if (/create an automation/i.test(msg.text)) {
    st.pendingTool = "create_scheduled_task";
    send("tool_proposed", { name: "create_scheduled_task", arguments: {} });
    send("permission_required", {
      name: "create_scheduled_task",
      arguments: {
        title: "Weekly digest",
        instructions: "Summarize the week and post it.",
        cron: "0 9 * * 1",
        permissions: [
          { tool: "send_message", target: "slack:T1/C1", access: "write" },
          { tool: "github_list_commits", target: "rohit/agent-platform", access: "read" },
        ],
      },
      reason: "",
      category: "automation",
    });
    return true;
  }
  // A reasoning model's turn: thinking deltas tick in slowly, then the answer —
  // the assistant_message carries the full trace like the real engine's payload.
  if (/think hard/i.test(msg.text)) {
    const thoughts = ["Weighing options. ", "Comparing tradeoffs. ", "Settling it. "];
    let tick = 0;
    const timer = setInterval(() => {
      if (tick < thoughts.length) {
        send("reasoning_delta", { text: thoughts[tick] });
        tick += 1;
        return;
      }
      clearInterval(timer);
      send("assistant_delta", { text: "Decision made." });
      send("assistant_message", {
        text: "Decision made.",
        reasoning: thoughts.join(""),
      });
      send("turn_done");
    }, 120);
    return true;
  }
  // Auto-compaction (OPE-27): the server signals `compacting` (the transient
  // spinner label), summarizes for a beat, then emits the marker and the turn
  // continues normally — the divider must render inline.
  if (/compact the context/i.test(msg.text)) {
    send("compacting", {});
    setTimeout(() => {
      send("compacted", { text: "Context compacted — earlier turns were summarized" });
      send("assistant_message", { text: "Still on it — continuing where I left off." });
      send("turn_done");
    }, 400);
    return true;
  }
  // A turn that dies on a provider error; the follow-up {type:"retry"} recovers.
  if (/fail the turn/i.test(msg.text)) {
    send("error", { error: "model unreachable" });
    send("turn_done");
    return true;
  }
  // A deliberately SLOW multi-second stream (~40 ticks × 120ms) so specs can
  // interact mid-turn — the follow/pin scroll contract (FB-004) is untestable
  // against the instant echo below.
  if (/stream the epic/i.test(msg.text)) {
    let ticks = 0;
    const line = "The epic scrolls ever onward, line upon line upon line. ";
    st.epicTimer = setInterval(() => {
      ticks += 1;
      send("assistant_delta", { text: line.repeat(3) + "\n\n" });
      if (ticks >= 40) {
        clearInterval(st.epicTimer!);
        st.epicTimer = null;
        send("assistant_message", { text: ("The epic concludes. " + line).repeat(20) });
        send("turn_done");
      }
    }, 120);
    return true;
  }
  return false;
}
