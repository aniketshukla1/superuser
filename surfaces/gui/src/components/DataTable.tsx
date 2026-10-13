/** A table the agent emitted as DATA, drawn in the conversation.
 *
 * Markdown tables already work and are the right answer for three rows of words. They
 * stop being the right answer the moment the table is a RESULT: markdown cannot say
 * "this column is a number", so nothing can right-align it, sort by it, or hand it to
 * a spreadsheet. Carrying the rows instead of a picture of the rows is what makes all
 * three possible — the same trade `Chart` already makes.
 *
 * Rendered as data: real cells, no `dangerouslySetInnerHTML`. A table spec is model
 * output derived from whatever the agent read, which may be a web page.
 */
import { useMemo, useRef, useState } from "react";
import { Icon, ICON } from "./Icon";

export type TableColumn = {
  field: string;
  label?: string;
  /** `number` right-aligns, sorts numerically and formats with grouping. */
  kind?: "text" | "number";
};

export type TableSpec = {
  title?: string;
  /** What the table shows, in the agent's words. */
  note?: string;
  columns: TableColumn[];
  rows: Record<string, string | number | null>[];
  /** Where the values came from — required of the agent and checked against what its
   *  run actually read (core::gates::grounding), same as a chart's. */
  source?: string;
};

/** Validate an already-parsed value. Null for anything we cannot draw, so the caller
 *  can fall back to showing the raw JSON rather than an empty block. */
export function asTableSpec(v: unknown): TableSpec | null {
  if (!v || typeof v !== "object") return null;
  const t = v as TableSpec;
  if (!Array.isArray(t.columns) || t.columns.length === 0) return null;
  if (t.columns.some((c) => !c || typeof c.field !== "string" || !c.field.trim())) return null;
  if (!Array.isArray(t.rows) || t.rows.length === 0) return null;
  return t;
}

const isNum = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);

/** Cell text. Numbers get thousands separators; null is an em dash, because an empty
 *  cell and a zero must not look the same. */
function display(v: unknown, kind?: string): string {
  if (v === null || v === undefined || v === "") return "—";
  if (kind === "number" || isNum(v)) {
    const n = typeof v === "number" ? v : Number(String(v).replace(/,/g, ""));
    if (Number.isFinite(n)) return n.toLocaleString();
  }
  return String(v);
}

export function DataTable({ spec }: { spec: TableSpec }) {
  const [sort, setSort] = useState<{ field: string; dir: 1 | -1 } | null>(null);
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const kindOf = (field: string) => spec.columns.find((c) => c.field === field)?.kind;

  const rows = useMemo(() => {
    if (!sort) return spec.rows;
    const numeric = kindOf(sort.field) === "number";
    // A copy: sorting must not reorder the spec, which is also what "copy as CSV"
    // reads. The CSV follows the order ON SCREEN, which is what you asked for by
    // sorting it, so it reads from this.
    return [...spec.rows].sort((a, b) => {
      const [x, y] = [a[sort.field], b[sort.field]];
      if (x === null || x === undefined || x === "") return 1; // blanks sink, either direction
      if (y === null || y === undefined || y === "") return -1;
      if (numeric) return (Number(x) - Number(y)) * sort.dir;
      return String(x).localeCompare(String(y)) * sort.dir;
    });
  }, [spec.rows, sort]);

  const toggle = (field: string) =>
    setSort((s) => (s?.field === field ? (s.dir === 1 ? { field, dir: -1 } : null) : { field, dir: 1 }));

  const copyCsv = async () => {
    // Every field quoted rather than guessing which need it: a value carrying a comma,
    // a quote or a newline is the case that corrupts a CSV silently.
    const esc = (v: unknown) => `"${String(v ?? "").replace(/"/g, '""')}"`;
    const head = spec.columns.map((c) => esc(c.label ?? c.field)).join(",");
    const body = rows.map((r) => spec.columns.map((c) => esc(r[c.field])).join(",")).join("\n");
    try {
      await navigator.clipboard.writeText(`${head}\n${body}\n`);
      setCopied(true);
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopied(false), 1400);
    } catch {
      /* a denied clipboard is not worth an error card over */
    }
  };

  return (
    <figure className="datatable" data-testid="datatable">
      <div className="datatable-bar">
        <figcaption className="datatable-title">{spec.title || "Table"}</figcaption>
        <button
          className="datatable-copy"
          onClick={copyCsv}
          aria-label={copied ? "Copied" : "Copy as CSV"}
          title={copied ? "Copied" : "Copy as CSV"}
          data-testid="datatable-copy"
        >
          <Icon name={copied ? "check" : "copy"} size={ICON.md} />
        </button>
      </div>
      <div className="datatable-scroll">
        <table>
          <thead>
            <tr>
              {spec.columns.map((c) => {
                const active = sort?.field === c.field;
                return (
                  <th
                    key={c.field}
                    className={c.kind === "number" ? "is-num" : undefined}
                    aria-sort={active ? (sort!.dir === 1 ? "ascending" : "descending") : "none"}
                  >
                    <button
                      className={"datatable-sort" + (active ? " is-on" : "")}
                      onClick={() => toggle(c.field)}
                      // Says what the NEXT press does, like the topbar's toggles.
                      title={
                        active
                          ? sort!.dir === 1
                            ? `Sort ${c.label ?? c.field} descending`
                            : `Stop sorting by ${c.label ?? c.field}`
                          : `Sort by ${c.label ?? c.field}`
                      }
                    >
                      {c.label ?? c.field}
                      <span className="datatable-caret" aria-hidden>
                        {active ? (sort!.dir === 1 ? "↑" : "↓") : ""}
                      </span>
                    </button>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={i}>
                {spec.columns.map((c) => (
                  <td key={c.field} className={c.kind === "number" ? "is-num" : undefined}>
                    {display(r[c.field], c.kind)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {spec.note && <p className="datatable-note">{spec.note}</p>}
      {spec.source && <p className="datatable-source">Source: {spec.source}</p>}
    </figure>
  );
}
