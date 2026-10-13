import { columnLabel } from "./externalChangeAction";

type XLSXModule = typeof import("xlsx");

/** xlsx → Univer snapshot: one sheet per workbook sheet, formulas kept as formulas. */
export function workbookToSnapshot(XLSX: XLSXModule, wb: import("xlsx").WorkBook) {
  const sheets: Record<string, unknown> = {};
  const sheetOrder: string[] = [];
  wb.SheetNames.forEach((name, i) => {
    const ws = wb.Sheets[name];
    const ref = ws["!ref"] || "A1";
    const r = XLSX.utils.decode_range(ref);
    const cellData: Record<number, Record<number, unknown>> = {};
    for (let row = r.s.r; row <= r.e.r; row++) {
      for (let col = r.s.c; col <= r.e.c; col++) {
        const cell = ws[XLSX.utils.encode_cell({ r: row, c: col })] as
          | { v?: unknown; f?: string }
          | undefined;
        if (!cell || (cell.v == null && !cell.f)) continue;
        cellData[row] ??= {};
        // A formula is carried as a formula, not as its stale cached value —
        // Univer recomputes it, which is also how the blank-formula problem
        // in notes/plans/WORKSPACE-PLAN.md stops being visible to the user.
        cellData[row][col] = cell.f ? { f: "=" + cell.f } : { v: cell.v as string | number | boolean };
      }
    }
    const id = `sheet-${i}`;
    sheetOrder.push(id);
    sheets[id] = {
      id,
      name,
      rowCount: Math.max(r.e.r + 1, 100),
      columnCount: Math.max(r.e.c + 1, 20),
      cellData,
    };
  });
  return { sheets, sheetOrder };
}

/** A SheetValueChanged event → the cell writes it implies. */
export function editsFromChange(params: unknown) {
  const p = params as { effectedRanges?: unknown[] };
  const edits: { cell: string; value: string }[] = [];
  let sheetName = "";
  for (const raw of p.effectedRanges ?? []) {
    const range = raw as {
      getRow: () => number;
      getColumn: () => number;
      getValues: () => unknown[][];
      getFormulas: () => string[][];
      getSheetName?: () => string;
    };
    const top = range.getRow();
    const left = range.getColumn();
    const values = range.getValues() ?? [];
    const formulas = range.getFormulas() ?? [];
    sheetName = range.getSheetName?.() ?? sheetName;
    values.forEach((row, dr) =>
      row.forEach((v, dc) => {
        const f = formulas[dr]?.[dc];
        const shown = v == null ? "" : String(v);
        // The formula is the source of truth where there is one; writing the
        // computed value instead would quietly turn a formula into a constant.
        // But the value rides along as the cached result, because a formula
        // saved without one is invisible to every reader that doesn't
        // recalculate — including SheetJS, which is how this component reads
        // the file back. Without it, a formula vanished on reopen.
        edits.push({
          cell: columnLabel(left + dc) + (top + dr + 1),
          value: f ? f : shown,
          ...(f && shown !== "" ? { result: shown } : {}),
        });
      }),
    );
  }
  return { edits, sheetName };
}
