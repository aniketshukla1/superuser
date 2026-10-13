import { describe, expect, it } from "vitest";
import { heatmap } from "./WorkspaceStats";

// The grid asks for as many week-columns as fit in its measured width, so the
// builder has to hand back exactly that many. It used to return
// `weeks * 7 + today.getDay() + 1` — a whole extra partial column — which was
// invisible while the grid scrolled sideways and became a silently cropped column
// the moment it stopped. That is the bug this file exists for.
describe("heatmap", () => {
  it("returns exactly the columns it was asked for", () => {
    for (const weeks of [1, 4, 9, 13, 19, 22, 34]) {
      const cells = heatmap([], weeks);
      expect(cells.length, `${weeks} weeks`).toBe(weeks * 7);
    }
  });

  it("ends on the week containing today", () => {
    const cells = heatmap([], 8);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    // Somewhere in the final column is today itself — the grid is about now, not
    // about a window that stopped last Saturday.
    const lastColumn = cells.slice(-7).map((c) => c.key);
    const key = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
    expect(lastColumn).toContain(key);
  });

  it("rows are weekdays, so every column starts on a Sunday", () => {
    const cells = heatmap([], 6);
    for (let i = 0; i < cells.length; i += 7) {
      expect(new Date(cells[i].key + "T00:00:00").getDay(), `cell ${i}`).toBe(0);
    }
  });

  it("marks days after today as future rather than as zero activity", () => {
    // A day that hasn't happened is not a quiet day; the CSS hides these, and
    // counting them as zero would make the current week look like a slump.
    const cells = heatmap([], 4);
    const future = cells.filter((c) => c.future);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    for (const c of future) expect(new Date(c.key + "T00:00:00").getTime()).toBeGreaterThan(today.getTime());
  });

  it("carries counts through for the days it covers", () => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const key = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
    const cells = heatmap([{ date: key, count: 7 }], 3);
    expect(cells.find((c) => c.key === key)?.count).toBe(7);
  });
});
