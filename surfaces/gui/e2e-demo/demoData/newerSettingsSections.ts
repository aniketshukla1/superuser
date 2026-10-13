// ---- the newer Settings sections, in the shapes their endpoints send ----
export const SETTINGS_SECTIONS: Record<string, unknown> = {
  "/v1/chief-of-staff": {
    priorities: "Close the Acme renewal\nShip checkout 2.41 without a latency regression\nHire two backend engineers",
    sources: [
      { source: "Email", items: 38 },
      { source: "Calendar", items: 12 },
      { source: "GitHub", items: 27 },
      { source: "Slack", items: 54 },
    ],
    routine: true,
  },
  "/v1/perf": {
    connected: true,
    account: "Prometheus 3.13.1 · prometheus.northwind.internal:9090",
    signals: 'Checkout p95: histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket{service="checkout"}[5m])))\nError rate: sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))',
    count: 2,
    problems: [],
    routine: true,
  },
  "/v1/expenses": { connected: true, intake: true, policy: "Meals under $60 a person\nFlights in economy under 6 hours", receipts: 41, this_month: 17, read_today: 6, daily_cap: 200, ledger: "~/Superuser/expenses/ledger.csv", routine: true },
  "/v1/email-bot": { connected: true, address: "sam@northwind.example", inbound: true, allowed: "dana@acme.example\n*@northwind.example", allowed_count: 2, live: true, problem: null, drafted_today: 3, daily_cap: 40 },
  "/v1/bug-repro": { staging: "https://staging.northwind.example", browser: true, on_this_mac: false, loopback: false, local_only: false },
};
