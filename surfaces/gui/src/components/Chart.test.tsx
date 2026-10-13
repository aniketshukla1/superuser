import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { Chart } from "./Chart";

afterEach(cleanup);

const axis = (container: HTMLElement) => [...container.querySelectorAll(".chart-axis")].map((n) => n.textContent);

describe("Chart scale", () => {
  it("keeps the largest value inside the plot", () => {
    // 694 is not a round number, so the axis has to run on to the next tick above it (800).
    // It stopped at 600, and every point above that was drawn off the top of the chart.
    const { container } = render(
      <Chart
        spec={{
          type: "line",
          x: { field: "t" },
          y: { field: "v" },
          data: [
            { t: "13:50", v: 412 },
            { t: "14:10", v: 694 },
          ],
        }}
      />,
    );
    expect(axis(container)).toContain("800");
    const ys = container
      .querySelector("polyline")!
      .getAttribute("points")!
      .split(" ")
      .map((p) => Number(p.split(",")[1]));
    // 10 is the plot's top edge (PAD.t); a smaller y is above it.
    for (const y of ys) expect(y).toBeGreaterThanOrEqual(10);
  });

  it("adds no tick above a maximum that is already round", () => {
    const { container } = render(
      <Chart
        spec={{
          type: "bar",
          x: { field: "t" },
          y: { field: "v" },
          data: [
            { t: "a", v: 300 },
            { t: "b", v: 600 },
          ],
        }}
      />,
    );
    expect(axis(container)).toContain("600");
    expect(axis(container)).not.toContain("800");
  });
});
