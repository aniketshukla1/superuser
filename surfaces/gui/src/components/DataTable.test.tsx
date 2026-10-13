import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { asTableSpec, DataTable, type TableSpec } from "./DataTable";
import { Transcript } from "./Transcript";
import type { Item } from "../types";

afterEach(cleanup);

const SPEC: TableSpec = {
  title: "Revenue by region",
  note: "North carried the quarter.",
  columns: [
    { field: "region", label: "Region" },
    { field: "revenue", label: "Revenue", kind: "number" },
  ],
  rows: [
    { region: "South", revenue: 90 },
    { region: "North", revenue: 1200 },
    { region: "East", revenue: null },
  ],
};

const bodyRows = () =>
  within(screen.getByTestId("datatable")).getAllByRole("row").slice(1); // drop the header row
const cells = () => bodyRows().map((r) => within(r).getAllByRole("cell").map((c) => c.textContent));

describe("a table spec", () => {
  it("refuses what it cannot draw, rather than rendering an empty frame", () => {
    expect(asTableSpec(null)).toBeNull();
    expect(asTableSpec({ columns: [], rows: [{ a: 1 }] })).toBeNull();
    expect(asTableSpec({ columns: [{ field: "a" }], rows: [] })).toBeNull();
    // A column with no field cannot address a cell, so the whole table is unreadable.
    expect(asTableSpec({ columns: [{ field: " " }], rows: [{ a: 1 }] })).toBeNull();
    expect(asTableSpec(SPEC)).toBeTruthy();
  });
});

describe("a table in the conversation", () => {
  it("formats numbers and distinguishes empty from zero", () => {
    render(<DataTable spec={SPEC} />);
    expect(cells()).toEqual([
      ["South", "90"],
      ["North", "1,200"],
      // An em dash, not "0" and not blank: a missing value and a zero are different
      // facts, and a table that shows them the same way is lying about the data.
      ["East", "—"],
    ]);
    render(<DataTable spec={{ ...SPEC, rows: [{ region: "Z", revenue: 0 }] }} />);
    expect(screen.getAllByTestId("datatable")[1].textContent).toContain("0");
  });

  it("sorts by a column, then reverses, then stops", () => {
    render(<DataTable spec={SPEC} />);
    const header = () => screen.getByRole("button", { name: /Revenue/ });

    fireEvent.click(header());
    // Numeric, so 90 before 1,200 — a string sort would put "1,200" first.
    expect(cells().map((c) => c[0])).toEqual(["South", "North", "East"]);

    fireEvent.click(header());
    expect(cells().map((c) => c[0])).toEqual(["North", "South", "East"]);

    // A third press clears it and the spec's own order comes back.
    fireEvent.click(header());
    expect(cells().map((c) => c[0])).toEqual(["South", "North", "East"]);
  });

  it("keeps blanks at the bottom whichever way it is sorted", () => {
    // Otherwise the empty row leads the ascending sort and the table opens on nothing.
    render(<DataTable spec={SPEC} />);
    const header = () => screen.getByRole("button", { name: /Revenue/ });
    fireEvent.click(header());
    expect(cells()[2][0]).toBe("East");
    fireEvent.click(header());
    expect(cells()[2][0]).toBe("East");
  });

  it("copies CSV in the order on screen, quoting every field", async () => {
    const writeText = vi.fn(async () => {});
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    render(
      <DataTable
        spec={{
          ...SPEC,
          // The value that silently corrupts a CSV if you only quote "when needed".
          rows: [{ region: 'North, "upper"', revenue: 1200 }],
        }}
      />,
    );
    screen.getByTestId("datatable-copy").click();
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith('"Region","Revenue"\n"North, ""upper""","1200"\n'),
    );
    vi.unstubAllGlobals();
  });
});

describe("visuals in a turn", () => {
  it("draws a table the agent emitted, in the order it was produced", () => {
    // Charts and tables share one ordered pass. Collecting by kind would group them
    // and quietly rearrange the argument the turn was making.
    const items: Item[] = [
      { kind: "user", text: "break down revenue" },
      {
        kind: "tool",
        id: "t1",
        name: "table",
        args: { spec: SPEC, source: "quarterly.csv" },
        status: "ok",
      },
      { kind: "assistant", text: "North carried it." },
    ];
    render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(screen.getByTestId("transcript-table")).toBeTruthy();
    expect(screen.getByText("Revenue by region")).toBeTruthy();
    // The provenance rides along, as it does for a chart.
    expect(screen.getByText(/quarterly\.csv/)).toBeTruthy();
  });

  it("ignores a refused table", () => {
    // A refused call carries a spec too, and drawing it would render exactly the
    // fabricated figures the gate just stopped.
    const items: Item[] = [
      { kind: "user", text: "x" },
      { kind: "tool", id: "t1", name: "table", args: { spec: SPEC }, status: "error" },
    ];
    render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(screen.queryByTestId("transcript-table")).toBeNull();
  });
});
