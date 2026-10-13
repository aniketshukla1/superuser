import { ago } from "./FromSource";

// ---- Privacy ▸ Network: what left this machine ----
export const host = (h: string, requests: number, sent: number, purposes: string[], minutes: number, blocked = 0) => ({
  host: h,
  requests,
  sent,
  blocked,
  last: ago(minutes),
  purposes,
});
export const EGRESS = {
  local_only: false,
  hosts: [
    host("api.github.com", 42, 18_400, ["github"], 6),
    host("imap.fastmail.com", 31, 0, ["email"], 12),
    host("slack.com", 17, 22_100, ["slack"], 14),
    host("prometheus.northwind.internal", 64, 31_800, ["prometheus"], 2),
    host("northwind.atlassian.net", 9, 4_100, ["jira"], 58),
    host("duckduckgo.com", 3, 900, ["web"], 190),
  ],
  requests: [
    { at: ago(2), purpose: "prometheus", method: "GET", host: "prometheus.northwind.internal", path: "/api/v1/query_range", sent: 512, status: 200, error: null, blocked: false },
    { at: ago(2), purpose: "prometheus", method: "GET", host: "prometheus.northwind.internal", path: "/api/v1/alerts", sent: 0, status: 200, error: null, blocked: false },
    { at: ago(6), purpose: "github", method: "GET", host: "api.github.com", path: "/repos/northwind/checkout/pulls", sent: 0, status: 200, error: null, blocked: false },
    { at: ago(12), purpose: "email", method: "IMAP", host: "imap.fastmail.com", path: ":993", sent: 0, status: null, error: null, blocked: false },
    { at: ago(14), purpose: "slack", method: "POST", host: "slack.com", path: "/api/conversations.history", sent: 96, status: 200, error: null, blocked: false },
  ],
};

export const EGRESS_LOCAL_ONLY = {
  local_only: true,
  hosts: [
    host("api.github.com", 3, 0, ["github"], 1, 3),
    host("slack.com", 2, 0, ["slack"], 1, 2),
    host("api.anthropic.com", 1, 0, ["model"], 2, 1),
  ],
  requests: [
    { at: ago(1), purpose: "github", method: "GET", host: "api.github.com", path: "/repos/northwind/checkout/pulls", sent: 0, status: null, error: "Local-only mode is on, so nothing may leave this machine", blocked: true },
    { at: ago(1), purpose: "slack", method: "POST", host: "slack.com", path: "/api/conversations.history", sent: 0, status: null, error: "Local-only mode is on, so nothing may leave this machine", blocked: true },
    { at: ago(2), purpose: "model", method: "POST", host: "api.anthropic.com", path: "/v1/messages", sent: 0, status: null, error: "Local-only mode is on, so nothing may leave this machine", blocked: true },
  ],
};
