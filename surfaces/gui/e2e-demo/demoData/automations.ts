import { NOW, ago } from "./FromSource";

// ---- Automations: routines, as the sidebar's Scheduled band and the Automations page show them ----
export const routine = (id: string, title: string, agent: string, schedule: string, cron: string, extra: Record<string, unknown> = {}) => ({
  id,
  title,
  instructions: "",
  schedule,
  schedule_raw: { kind: "cron", cron, fire_at: null, timezone: "local" },
  workspace: "",
  agent,
  enabled: true,
  next_run: NOW + 60 * 60 * 14,
  last_run: ago(60 * 3),
  last_status: "ok",
  run_count: 12,
  notify_on_completion: true,
  always_allowed: [],
  unseen_runs: 0,
  unseen_failed: false,
  seen_runs_at: 0,
  ...extra,
});
export const AUTOMATIONS = [
  routine("task-digest", "Morning digest", "chief", "Weekdays at ~8:00 AM", "0 8 * * 1-5", { unseen_runs: 1 }),
  routine("task-health", "Weekly health report", "perf", "Every Monday at ~9:30 AM", "30 9 * * 1"),
  routine("task-expenses", "Weekly expense summary", "expenses", "Every Friday at ~5:00 PM", "0 17 * * 5"),
];
