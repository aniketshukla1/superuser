// What the film shows Superuser doing: the work on the screens that fly out of the dark,
// and the lines running down the terminals behind them. A showreel, labelled as one (the
// Skip control says "intro"); the handles are the built-in assistants' real ones, so
// what you see is what `@coder` or `@hr` would actually be doing.

/** What a screen shows: a document being written or a chart being drawn. */
export type Art =
  | "code"
  | "email"
  | "bars"
  | "line"
  | "donut"
  | "kanban"
  | "checklist"
  | "ledger"
  | "terminal"
  | "calendar"
  | "kpi";

export interface Task {
  /** The assistant's handle — `@who`. */
  who: string;
  doing: string;
  art: Art;
  /** What the machine says when it locks onto this one. */
  verdict: string;
}

export const TASKS: Task[] = [
  { who: "coder", doing: "writing the retry logic", art: "code", verdict: "tests pass" },
  { who: "email", doing: "drafting a reply to Dana", art: "email", verdict: "draft · needs your ok" },
  { who: "analyst", doing: "Q3 revenue report", art: "bars", verdict: "4 charts ready" },
  { who: "ops", doing: "watching api latency", art: "line", verdict: "p99 41 ms · nominal" },
  { who: "pm", doing: "ranking the Q4 roadmap", art: "kanban", verdict: "12 bets ranked" },
  { who: "hr", doing: "onboarding two new hires", art: "checklist", verdict: "7 of 9 done" },
  { who: "expenses", doing: "matching receipts", art: "ledger", verdict: "reconciled · $2,840" },
  { who: "qa", doing: "running the test suite", art: "terminal", verdict: "248 passed" },
  { who: "chief", doing: "your morning brief", art: "calendar", verdict: "2 decisions owed" },
  { who: "finance", doing: "forecasting cash flow", art: "kpi", verdict: "runway 19 months" },
  { who: "analyst", doing: "churn by plan", art: "donut", verdict: "churn 2.1%" },
  { who: "support", doing: "answering ticket #4182", art: "email", verdict: "reply drafted" },
  { who: "security", doing: "auditing dependencies", art: "terminal", verdict: "0 critical" },
  { who: "perf", doing: "chasing a slow checkout", art: "line", verdict: "hotspot found" },
  { who: "writer", doing: "release notes for 1.0", art: "code", verdict: "ready to review" },
  { who: "lead", doing: "splitting the migration", art: "kanban", verdict: "5 tasks assigned" },
];

/** The task the film ends on: captured, and stamped done. */
export const FINISHED = { who: "analyst", doing: "Q3 revenue report", art: "bars" as Art };

export type Tone = "dim" | "bright" | "ok" | "warn";
export interface TermLine {
  text: string;
  tone: Tone;
}

const FILES = ["billing/invoice.ts", "auth/session.rs", "api/retry.ts", "sync/queue.go", "ui/Chart.tsx", "db/migrate.sql"];
const HOSTS = ["api.internal", "auth.svc", "billing-db", "queue-01", "edge-lb", "search"];
const TOPICS = ["launch plan", "Q3 targets", "Dana's contract", "hiring loop", "pricing test"];
const WHO = ["coder", "email", "analyst", "ops", "pm", "hr", "expenses", "chief", "finance", "support"];

/** A terminal's worth of work going on, faster than anyone could read. */
export function terminalLines(r: () => number, count: number): TermLine[] {
  const pick = <T,>(xs: T[]) => xs[Math.floor(r() * xs.length)];
  const n = (lo: number, hi: number) => Math.floor(lo + r() * (hi - lo));
  const hex = (len: number) => Array.from({ length: len }, () => "0123456789abcdef"[n(0, 16)]).join("");
  let clock = 0;
  return Array.from({ length: count }, () => {
    clock += 0.004 + r() * 0.02;
    const ts = `[${clock.toFixed(3).padStart(7)}]`;
    const roll = r();
    if (roll < 0.1) return { text: `0x${hex(8)} ${hex(2)} ${hex(2)} ${hex(2)} ${hex(2)} ${hex(2)} ${hex(2)}`, tone: "dim" };
    if (roll < 0.22) return { text: `${ts} coder    edit    src/${pick(FILES)}  +${n(3, 90)} −${n(0, 30)}`, tone: "bright" };
    if (roll < 0.32) return { text: `${ts} tests    run     ${n(40, 400)} passed  0 failed`, tone: "ok" };
    if (roll < 0.41) return { text: `${ts} email    draft   re: "${pick(TOPICS)}" → your ok`, tone: "warn" };
    if (roll < 0.5) return { text: `${ts} analyst  query   revenue.csv  ${n(1000, 90000).toLocaleString("en-US")} rows`, tone: "dim" };
    if (roll < 0.59) return { text: `${ts} ops      probe   ${pick(HOSTS)}  200  ${n(4, 240)}ms`, tone: "ok" };
    if (roll < 0.67) return { text: `${ts} brain    recall  "${pick(TOPICS)}"  ${n(2, 14)} memories`, tone: "dim" };
    if (roll < 0.75) return { text: `${ts} git      commit  ${hex(7)} "${pick(["fix: retry on 429", "feat: export csv", "chore: bump deps"])}"`, tone: "bright" };
    if (roll < 0.83) return { text: `${ts} ask      @${pick(WHO)} → @${pick(WHO)}  "${pick(TOPICS)}?"`, tone: "bright" };
    if (roll < 0.91) return { text: `${ts} chart    render  ${pick(["revenue", "churn", "latency", "pipeline"])}.svg  ${n(3, 9)} series`, tone: "dim" };
    return { text: `${ts} hr       parse   resume_${n(10, 99)}.pdf  ok`, tone: "dim" };
  });
}
