import { fetch, httpBase, jsonBody, jsonOf, listOf } from "./neverRejectingJson";
import type { AuditDecision, AuditInitiator, AuditSubject } from "./waitForCloudSignIn";

/** One row of the activity trail. */
export interface AuditEvent {
  id: number;
  timestamp: string;
  tool: string;
  connector: string;
  stage: string;
  /** "ok", "error" or "refused". */
  status: string;
  approval: string;
  session_id: string;
  agent: string;
  workspace: string;
  args: Record<string, any>;
  result_preview: string;
  reason?: string;
  resource?: string;
  // The trail's second version; an older server leaves these out.
  at?: number;
  /** action.allowed, action.refused, action.failed, action.stopped, action.parked,
   *  action.declined, assistant.declined, turn.stalled, policy.loaded, policy.changed,
   *  audit.wiped, audit.retention, or a first-version kind such as "sync". */
  event?: string;
  ok?: boolean;
  actor?: string;
  persona?: string;
  target?: string;
  detail?: string;
  initiator?: AuditInitiator | null;
  /** The run happened with nobody watching: a schedule, a watch, a message, the API. */
  unwatched?: boolean;
  subject?: AuditSubject | null;
  decision?: AuditDecision | null;
}

export type AuditQuery = {
  limit?: number;
  session_id?: string;
  connector?: string;
  tool?: string;
  /** An event name or a family of them, e.g. "action". */
  event?: string;
  nobody_watching?: boolean;
  refused?: boolean;
  /** Seconds since the epoch. */
  since?: number;
};

// The order the server has always received them in.
const QUERY_ORDER = ["limit", "session_id", "connector", "tool", "event", "nobody_watching", "refused", "since"] as const;

export async function getAudit(params: AuditQuery = {}): Promise<AuditEvent[]> {
  const query = new URLSearchParams();
  for (const key of QUERY_ORDER) {
    const value = params[key];
    if (value) query.set(key, value === true ? "1" : String(value));
  }
  const qs = query.toString();
  return listOf(await fetch(`${httpBase()}/v1/audit${qs ? `?${qs}` : ""}`), "events");
}

/** Whether the trail's hash chain still holds together. */
export interface AuditIntegrity {
  intact: boolean;
  rows: number;
  linked: number;
  legacy: number;
  problem?: string | null;
}

export async function verifyAudit(): Promise<AuditIntegrity | null> {
  const body = await jsonOf(await fetch(`${httpBase()}/v1/audit/verify`));
  return typeof body.intact === "boolean" ? (body as AuditIntegrity) : null;
}

type RetentionReply = { ok: boolean; audit_retention_days?: number; removed?: number; error?: string };

export async function setAuditRetention(days: number): Promise<RetentionReply> {
  const res = await fetch(`${httpBase()}/v1/settings/audit-retention`, jsonBody("POST", { days }));
  return (await jsonOf(res)) as RetentionReply;
}
