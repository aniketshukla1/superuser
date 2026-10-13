import { fetch, httpBase, jsonOf } from "./neverRejectingJson";

// -- Egress: what actually left this machine ---------------------------------
// Every outbound request goes through one chokepoint (`core/egress.rs`), so this
// is the whole picture rather than the parts someone remembered to log.

export interface EgressRequest {
  at: number;
  /** Who wanted it: a connector id, "model", "web", "mcp". */
  purpose: string;
  method: string;
  host: string;
  /** Path only — query strings are never recorded, since they carry tokens. */
  path: string;
  sent: number;
  status: number | null;
  error: string | null;
  /** True when local-only mode stopped it, so nothing was actually sent. */
  blocked: boolean;
}

export interface EgressHost {
  host: string;
  requests: number;
  sent: number;
  blocked: number;
  last: number;
  purposes: string[];
}

export interface EgressState {
  requests: EgressRequest[];
  hosts: EgressHost[];
  local_only: boolean;
}

export async function getEgress(): Promise<EgressState> {
  const res = await fetch(`${httpBase()}/v1/egress`);
  if (!res.ok) return { requests: [], hosts: [], local_only: false };
  const j = await jsonOf(res);
  return { requests: j.requests ?? [], hosts: j.hosts ?? [], local_only: j.local_only ?? false };
}

export async function clearEgress(): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/egress/clear`, { method: "POST" });
  return res.ok && (await jsonOf(res)).ok === true;
}

export async function setLocalOnly(enabled: boolean): Promise<boolean | null> {
  const res = await fetch(`${httpBase()}/v1/egress/local-only`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ enabled }),
  });
  if (!res.ok) return null;
  const j = await jsonOf(res);
  return j.ok ? j.local_only : null;
}
