import { ago } from "./FromSource";

// ---- the hero conversation: Product Performance finds a hotspot ----
export const PROM = "http://prometheus.northwind.internal:9090/graph?g0.expr=";
export const series = [
  ["13:30", 402, 118], ["13:40", 409, 121], ["13:50", 415, 117], ["14:00", 411, 120],
  ["14:10", 655, 119], ["14:20", 681, 122], ["14:30", 692, 118], ["14:40", 688, 121],
  ["14:50", 694, 120], ["15:00", 686, 119],
].map(([t, confirm, cart]) => ({ t, confirm, cart }));

export const HERO_MESSAGES = [
  { role: "user", content: "Checkout got slower since yesterday's 2.41 release. Find where, with evidence.", at: ago(9) },
  {
    role: "assistant",
    at: ago(2),
    items: [
      { kind: "tool", name: "metrics_find", args: { text: "checkout duration" }, status: "ok" },
      { kind: "tool", name: "metrics_compare", args: { query: 'histogram_quantile(0.95, sum by (le, handler) (rate(http_request_duration_seconds_bucket{service="checkout"}[5m])))', change_at: "2026-09-29 14:02", window: "1h" }, status: "ok" },
      { kind: "tool", name: "metrics_compare", args: { query: 'histogram_quantile(0.95, sum by (le, version) (rate(http_request_duration_seconds_bucket{handler="/checkout/confirm"}[5m])))', change_at: "2026-09-29 14:02", window: "1h" }, status: "ok" },
      { kind: "tool", name: "metrics_alerts", args: {}, status: "ok" },
      {
        kind: "tool",
        name: "chart",
        status: "ok",
        args: {
          source: "checkout-p95.csv",
          spec: {
            type: "line",
            title: "p95 latency by handler (ms)",
            note: "A step at the 14:02 deploy on /checkout/confirm. /cart does not move.",
            x: { field: "t", label: "29 Sep" },
            series: [
              { field: "confirm", label: "/checkout/confirm" },
              { field: "cart", label: "/cart" },
            ],
            data: series,
          },
        },
      },
    ],
    content: [
      "**Hotspot: `POST /checkout/confirm` on version 2.41.0.** Its p95 went from 412 ms to 688 ms at the 14:02 deploy (+67%), a sustained step rather than a spike. The other four handlers stayed inside their usual variation.",
      "",
      "**Facts**",
      `- \`/checkout/confirm\` p95: 412 ms → 688 ms, moved clearly ([query](${PROM}confirm_p95))`,
      `- Only pods on \`2.41.0\` moved; pods still on \`2.40.3\` are unchanged ([query](${PROM}by_version))`,
      "- Error rate is flat at 0.21% and CPU saturation is flat. No alerts are firing.",
      "",
      "**Hypotheses** (not shown yet)",
      "- The new tax-service call runs inline. One slow trace from Tempo would settle it.",
      "- A missing index on `orders.confirmed_at`. `EXPLAIN` on the confirm query would settle it.",
      "",
      "The series I read are saved as `checkout-p95.csv`. I changed no alerts and no settings.",
    ].join("\n"),
  },
];
