// ---- Connectors: a few connected, Prometheus with the tools assistants use on it ----
export const CONNECTED: Record<string, string> = {
  prometheus: "Prometheus 3.13.1 · prometheus.northwind.internal:9090",
  github: "northwind",
  email: "sam@northwind.example",
  jira: "northwind.atlassian.net",
  linear: "Northwind",
};
export const read = (name: string, label: string, description: string) => ({ name, label, kind: "read", description, enabled: true, requires_approval: false });
export const PROMETHEUS_TOOLS = [
  read("metrics_find", "Find metrics", "Search metric names, with each one's help text and labels."),
  read("metrics_query", "Run a query", "Run a PromQL query for one moment — now, or any time before."),
  read("metrics_range", "Query a time range", "Average, min, max and last for each series a PromQL query returns over a range."),
  read("metrics_compare", "Compare before and after", "Which series moved at a release or config change, and by how much."),
  read("metrics_alerts", "Read alerts", "The alerts firing or pending now. Nothing is silenced or changed."),
  read("perf_report", "Health report", "Your Product Performance signals, compared with the window before."),
];
export const SYNC_TOOLS = [
  read("connector_search", "Search synced content", "Search what has been synced from this app into memory."),
  read("connector_sync", "Sync now", "Pull the latest from this app into memory."),
  read("connector_changes", "What's new", "What was added, changed or removed since an assistant last looked."),
];
