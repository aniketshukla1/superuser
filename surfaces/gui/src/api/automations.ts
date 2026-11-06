/** A scheduled task as the server reports it. */
export interface Automation {
  id: string;
  title: string;
  instructions: string;
  agent: string;
  workspace: string;
  /** What runs it: `null` means whatever the app would choose. The scheduler has always
   *  stored and honoured it; it just wasn't sent back, so a pinned automation looked
   *  unpinned. */
  model?: string | null;
  schedule: string;
  schedule_raw?: { kind: string; cron?: string | null; fire_at?: string | null; timezone?: string };
  enabled: boolean;
  notify_on_completion: boolean;
  next_run: number | null;
  last_run: number | null;
  last_status: string | null;
  run_count: number;
  /** UX-023 sidebar badge: runs since the user last opened this automation, and whether
   *  the newest of them failed (shown in the danger tint). */
  unseen_runs?: number;
  unseen_failed?: boolean;
  seen_runs_at?: number;
  /** Standing scoped approvals (§25): rules bound to a target that this automation may use
   *  without asking. `entry` is the stored form and the handle for revoking; `target` is
   *  null for older entries that name only a tool. */
  always_allowed: { entry: string; tool: string; target: string | null }[];
}

/** One run of a scheduled task. */
export interface AutomationRun {
  run_id: string;
  task_id: string;
  session_id: string;
  trigger: string;
  status: string;
  started_at: number;
  finished_at: number | null;
  result_text: string | null;
  error: string | null;
  artifacts: string[];
  /** Whether the run kept a transcript you can open. Scheduled runs only began saving one
   *  on 2026-08-26, so older runs have a result with no conversation behind it. */
  has_conversation?: boolean;
}
