import { MODEL, stamp } from "./FromSource";

// ---- conversations ----
export const session = (id: string, title: string, agent: string, minutes: number, extra: Record<string, unknown> = {}) => ({
  session_id: id,
  title,
  workspace: "",
  agent,
  model: MODEL,
  mode: "interactive",
  updated_at: stamp(minutes),
  messages: 4,
  pinned: false,
  archived: false,
  attention: 0,
  liveness: "idle",
  subscriptions: [],
  ...extra,
});

export const HERO = "perf-checkout";
export const SESSIONS = [
  session(HERO, "Checkout latency since the 2.41 release", "perf", 2, { pinned: true }),
  session("perf-weekly", "Weekly health report", "perf", 60 * 26),
  session("chief-digest", "Morning digest — Tuesday", "chief", 60 * 3, { attention: 1 }),
  session("asst-memo", "Draft the Q4 planning memo", "assistant", 60 * 5),
  session("asst-contract", "Summarise the vendor contract", "assistant", 60 * 30),
  session("email-replies", "Replies waiting for approval", "email", 60 * 7, { attention: 1 }),
  session("exp-sept", "September expense summary", "expenses", 60 * 28),
  session("coder-flaky", "Fix the flaky checkout test", "coder", 60 * 50),
];
