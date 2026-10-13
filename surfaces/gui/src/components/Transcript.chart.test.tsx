import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { Transcript } from "./Transcript";
import type { Item } from "../types";

afterEach(cleanup);

const SPEC = {
  type: "bar",
  title: "Revenue by quarter",
  note: "Q4 carried the year.",
  x: { field: "quarter" },
  y: { field: "revenue" },
  data: [
    { quarter: "Q1", revenue: 120 },
    { quarter: "Q2", revenue: 140 },
    { quarter: "Q4", revenue: 310 },
  ],
};

const turn = (over: Partial<Extract<Item, { kind: "tool" }>> = {}): Item[] => [
  { kind: "user", text: "chart the revenue" },
  { kind: "tool", id: "t0", name: "sheet_read", args: { path: "q.xlsx" }, status: "ok" },
  {
    kind: "tool",
    id: "t1",
    name: "chart",
    args: { path: "rev.chart.json", spec: SPEC, source: "q.xlsx" },
    status: "ok",
    ...over,
  },
  { kind: "assistant", text: "Q4 carried the year." },
];

describe("charts in the conversation", () => {
  it("draws the chart the agent made, without expanding the steps", () => {
    // The turn group renders COLLAPSED. A chart drawn inside it would be a chart
    // nobody sees, which is the whole reason it is a block of its own.
    render(<Transcript items={turn()} onApprove={vi.fn()} />);
    expect(screen.queryByTestId("turn-step")).toBeNull();
    expect(screen.getByTestId("transcript-chart")).toBeTruthy();
    expect(screen.getByText("Revenue by quarter")).toBeTruthy();
    // One bar per row, from the spec's own data.
    expect(screen.getByTestId("transcript-chart").querySelectorAll("rect").length).toBe(3);
  });

  it("shows where the numbers came from", () => {
    render(<Transcript items={turn()} onApprove={vi.fn()} />);
    expect(screen.getByTestId("chart-source").textContent).toContain("q.xlsx");
  });

  it("does not draw a chart the gates refused", () => {
    // A refused call still carries a spec. Drawing it would render exactly the
    // fabricated picture the refusal was for.
    render(<Transcript items={turn({ status: "error", preview: "error: `source` cites …" })} onApprove={vi.fn()} />);
    expect(screen.queryByTestId("transcript-chart")).toBeNull();
  });

  it("ignores a spec it cannot draw rather than rendering an empty frame", () => {
    const noData = { ...SPEC, data: [] };
    render(
      <Transcript items={turn({ args: { path: "x.chart.json", spec: noData } })} onApprove={vi.fn()} />,
    );
    expect(screen.queryByTestId("transcript-chart")).toBeNull();
  });

  it("draws the chart before the answer that explains it", () => {
    const { container } = render(<Transcript items={turn()} onApprove={vi.fn()} />);
    const html = container.innerHTML;
    const chart = html.indexOf("Revenue by quarter");
    const answer = html.lastIndexOf("Q4 carried the year.");
    // Both have to be FOUND, not just ordered: `indexOf` returns -1 for a chart that
    // never rendered, and -1 is dutifully less than everything.
    expect(chart).toBeGreaterThan(-1);
    expect(answer).toBeGreaterThan(-1);
    expect(chart).toBeLessThan(answer);
  });

  it("draws every chart in a turn, in the order they were made", () => {
    const two: Item[] = [
      { kind: "user", text: "chart both" },
      { kind: "tool", id: "a", name: "chart", args: { path: "a.chart.json", spec: { ...SPEC, title: "First" } }, status: "ok" },
      { kind: "tool", id: "b", name: "chart", args: { path: "b.chart.json", spec: { ...SPEC, title: "Second" } }, status: "ok" },
      { kind: "assistant", text: "Both drawn." },
    ];
    const { container } = render(<Transcript items={two} onApprove={vi.fn()} />);
    expect(screen.getAllByTestId("transcript-chart").length).toBe(2);
    expect(container.innerHTML.indexOf("First")).toBeLessThan(container.innerHTML.indexOf("Second"));
  });
});
