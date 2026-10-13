import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { ConnectorTools } from "./ManageTabs";
import { ToolsDisclosure } from "./connectors/ToolsDisclosure";
import type { Connector, ConnectorTool } from "../api";

const tool = (name: string, label: string, writes = false): ConnectorTool => ({
  name,
  label,
  kind: writes ? "write" : "read",
  description: `What ${label} does.`,
  enabled: true,
  requires_approval: writes,
});

// What `GET /v1/connectors` sends: the tools assistants reach each connector through.
const PROMETHEUS_TOOLS = [
  tool("metrics_find", "Find metrics"),
  tool("metrics_query", "Run a query"),
  tool("metrics_range", "Query a time range"),
  tool("metrics_compare", "Compare before and after"),
  tool("metrics_alerts", "Read alerts"),
  tool("perf_report", "Health report"),
];
const SLACK_TOOLS = [
  tool("connector_search", "Search synced content"),
  tool("connector_sync", "Sync now"),
  tool("connector_changes", "What's new"),
  tool("send_message", "Send a message", true),
];

const connector = (over: Partial<Connector>): Connector => ({
  name: "prometheus",
  title: "Prometheus",
  icon: "prometheus",
  blurb: "",
  auth: "token",
  two_way: false,
  channels: false,
  available: true,
  fields: [],
  instructions: [],
  connected: true,
  account: null,
  enabled: true,
  brand_color: "#e6522c",
  logo: "prometheus",
  allowed_users: [],
  tools: [],
  managed: false,
  managed_profile: false,
  ...over,
});

afterEach(cleanup);

describe("ConnectorTools", () => {
  it("lists the tools assistants use on a connector instead of saying it has none", () => {
    render(<ConnectorTools c={connector({ tools: PROMETHEUS_TOOLS })} />);
    expect(screen.queryByText("No tools for this connector yet.")).toBeNull();
    for (const t of PROMETHEUS_TOOLS) expect(screen.getByText(t.label)).toBeTruthy();
    expect(screen.getByText("metrics_query · read")).toBeTruthy();
    // A list, not switches: nothing on the server would act on a toggle.
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    // Read-only tools never wait for a yes, so none of them claims to.
    expect(screen.queryByText(/asks approval/)).toBeNull();
  });

  it("says a tool asks approval only when it does", () => {
    render(<ConnectorTools c={connector({ name: "slack", title: "Slack", tools: SLACK_TOOLS })} />);
    expect(screen.getByText("send_message · write · asks approval")).toBeTruthy();
    expect(screen.getByText("connector_sync · read")).toBeTruthy();
  });

  it("keeps the empty line for a connector nothing reaches yet", () => {
    render(<ConnectorTools c={connector({ name: "monday", title: "monday.com", available: false })} />);
    expect(screen.getByText("No tools for this connector yet.")).toBeTruthy();
  });
});

describe("ToolsDisclosure", () => {
  it("lists the same tools on the bespoke pages, with no switches", () => {
    render(<ToolsDisclosure c={connector({ name: "slack", title: "Slack", tools: SLACK_TOOLS })} />);
    expect(screen.getByText("4 tools assistants can use")).toBeTruthy();
    expect(screen.getByText("Send a message")).toBeTruthy();
    expect(screen.getByText("asks first")).toBeTruthy();
    expect(screen.getAllByText("read")).toHaveLength(3);
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
  });

  it("renders nothing for a connector with no tools", () => {
    const { container } = render(<ToolsDisclosure c={connector({ tools: [] })} />);
    expect(container.innerHTML).toBe("");
  });
});
