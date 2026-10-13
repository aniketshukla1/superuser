// Column addressing — the join between Univer's numeric row/column and the A1
// references `sheets::write_cells` takes. An off-by-one here silently writes the
// user's edit into the wrong cell of their file.
import { describe, expect, it } from "vitest";
import { columnLabel, externalChangeAction, toCsv } from "./UniverSheet";

describe("columnLabel", () => {
  it("covers the single letters", () => {
    expect(columnLabel(0)).toBe("A");
    expect(columnLabel(1)).toBe("B");
    expect(columnLabel(25)).toBe("Z");
  });

  it("rolls over into two letters the way spreadsheets do", () => {
    // The classic off-by-one: 26 is AA, not BA or AZ.
    expect(columnLabel(26)).toBe("AA");
    expect(columnLabel(27)).toBe("AB");
    expect(columnLabel(51)).toBe("AZ");
    expect(columnLabel(52)).toBe("BA");
    expect(columnLabel(701)).toBe("ZZ");
    expect(columnLabel(702)).toBe("AAA");
  });

  it("agrees with the cell references the server accepts", () => {
    // `sheets::is_cell_ref` takes /[A-Z]+[0-9]+/ — every label must satisfy it.
    for (const i of [0, 1, 25, 26, 100, 701, 702]) {
      expect(columnLabel(i) + "1").toMatch(/^[A-Z]+[0-9]+$/);
    }
  });
});

describe("when the file changes under an open editor", () => {
  it("does nothing when the workbook is the same", () => {
    expect(externalChangeAction({ changed: false, busy: false })).toBe("ignore");
    expect(externalChangeAction({ changed: false, busy: true })).toBe("ignore");
  });

  it("shows the agent's edit straight away when nobody is typing", () => {
    expect(externalChangeAction({ changed: true, busy: false })).toBe("adopt");
  });

  it("never rebuilds the editor under someone mid-keystroke", () => {
    // Rebuilding resets selection and undo. Offering the reload costs nothing —
    // the edit is already safely on disk either way.
    expect(externalChangeAction({ changed: true, busy: true })).toBe("offer");
  });
});

describe("serialising a sheet back to CSV", () => {
  it("leaves ordinary values alone", () => {
    expect(toCsv([["Service", "Monthly"], ["Notion", 10]])).toBe("Service,Monthly\nNotion,10\n");
  });

  it("quotes only what would otherwise change meaning", () => {
    // A comma or newline inside a field would silently become a new column or row.
    expect(toCsv([["a,b", "plain"]])).toBe('"a,b",plain\n');
    expect(toCsv([["two\nlines"]])).toBe('"two\nlines"\n');
    expect(toCsv([['say "hi"']])).toBe('"say ""hi"""\n');
  });

  it("writes empty cells as empty, not as the string null", () => {
    expect(toCsv([[null, undefined, "", 0]])).toBe(",,,0\n");
    // An empty sheet is an empty file, not a lone newline.
    expect(toCsv([])).toBe("");
  });
});
