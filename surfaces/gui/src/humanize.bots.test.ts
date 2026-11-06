import { describe, expect, it } from "vitest";
import { humanizeTool } from "./humanize";

// The bots' tools read as sentences in the transcript, not as their names.
const line = (name: string, args: unknown) => {
  const l = humanizeTool(name, args);
  return `${l.pre}${l.obj ?? ""}${l.post ?? ""}`;
};

describe("the bots' tools in plain words", () => {
  it("says what Product Performance and Talent Scout did", () => {
    expect(line("metrics_compare", { query: "q", change_at: "2026-11-05 14:02" })).toBe("Compared every series before and after 2026-11-05 14:02");
    expect(line("metrics_alerts", {})).toBe("Checked the alerts firing in Prometheus");
    expect(line("candidates_check", { candidates: [{}, {}, {}] })).toBe("Checked 3 candidates against your ATS and past shortlists");
    expect(line("candidates_shortlist", { role: "Senior SRE", candidates: [{}] })).toBe("Shortlisted 1 candidate for “Senior SRE”");
    expect(line("activity_since", { since: "this_week" })).toBe("Read what came in since this week");
  });

  it("never falls back to a tool's raw name for them", () => {
    for (const name of ["metrics_find", "metrics_query", "metrics_range", "perf_report", "expense_summary", "browser_console", "browser_screenshot"]) {
      expect(line(name, {})).not.toContain(name);
    }
  });
});
