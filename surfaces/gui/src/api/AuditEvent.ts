import { fetch, httpBase, jsonOf, listOf } from "./neverRejectingJson";
import type { AuditDecision, AuditInitiator, AuditSubject } from "./waitForCloudSignIn";

export interface AuditEvent {
  id: number;
  timestamp: string;
  session_id: string;
  agent: string;
  workspace: string;
  connector: string;
  tool: string;
  stage: string;
  /** ok · error · refused */
  status: string;
  approval: string;
  args: Record<string, any>;
  result_preview: string;
  reason?: string;
  resource?: string;
  // Version 2 of the trail. Absent from an older server.
  at?: number;
  /** action.allowed · action.refused · action.failed · action.stopped · action.parked ·
   *  action.declined · assistant.declined · turn.stalled · policy.loaded · policy.changed ·
   *  audit.wiped · audit.retention — or a first-version kind like "sync". */
  event?: string;
  actor?: string;
  target?: string;
  detail?: string;
  ok?: boolean;
  persona?: string;
  initiator?: AuditInitiator | null;
  /** Nobody was watching the run happen: a schedule, a watch, a message, the API. */
  unwatched?: boolean;
  subject?: AuditSubject | null;
  decision?: AuditDecision | null;
}

export async function getAudit(params: {
  limit?: number;
  session_id?: string;
  connector?: string;
  tool?: string;
  /** An event name or family, e.g. "action". */
  event?: string;
  nobody_watching?: boolean;
  refused?: boolean;
  /** Epoch seconds. */
  since?: number;
} = {}): Promise<AuditEvent[]> {
  const q = new URLSearchParams();
  if (params.limit) q.set("limit", String(params.limit));
  if (params.session_id) q.set("session_id", params.session_id);
  if (params.connector) q.set("connector", params.connector);
  if (params.tool) q.set("tool", params.tool);
  if (params.event) q.set("event", params.event);
  if (params.nobody_watching) q.set("nobody_watching", "1");
  if (params.refused) q.set("refused", "1");
  if (params.since) q.set("since", String(params.since));
  const res = await fetch(`${httpBase()}/v1/audit${q.toString() ? "?" + q.toString() : ""}`);
  return listOf(res, "events");
}

/** Whether the trail's chain still adds up. */
export interface AuditIntegrity {
  rows: number;
  linked: number;
  legacy: number;
  intact: boolean;
  problem?: string | null;
}

export async function verifyAudit(): Promise<AuditIntegrity | null> {
  const res = await fetch(`${httpBase()}/v1/audit/verify`);
  const body = await jsonOf(res);
  return typeof body.intact === "boolean" ? (body as AuditIntegrity) : null;
}

export async function setAuditRetention(days: number): Promise<{ ok: boolean; audit_retention_days?: number; removed?: number; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/settings/audit-retention`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ days }),
  });
  return jsonOf(res) as Promise<{ ok: boolean; audit_retention_days?: number; removed?: number; error?: string }>;
}
