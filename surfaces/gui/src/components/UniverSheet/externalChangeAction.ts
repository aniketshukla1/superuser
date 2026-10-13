/**
 * 0 → A, 25 → Z, 26 → AA.
 *
 * Univer reports a changed range by numeric row/column; `sheets::write_cells` takes
 * A1 references. This is the join between them, so an off-by-one here writes the
 * user's edit into the wrong cell of their file — hence the tests.
 */
export function columnLabel(index: number): string {
  let n = index;
  let out = "";
  while (n >= 0) {
    out = String.fromCharCode(65 + (n % 26)) + out;
    n = Math.floor(n / 26) - 1;
  }
  return out;
}

/**
 * What to do when the file changes underneath an open editor.
 *
 * Only ever called for *external* writes — our own saves don't re-read the file, so
 * a changed workbook means the agent (or a tool) wrote to it. Adopting it silently
 * is right when nobody is mid-keystroke, and wrong when they are: rebuilding the
 * editor resets selection and undo history, which is the same interruption Phase 3
 * decided against for the Browser tab.
 */
export function externalChangeAction(now: {
  /** The incoming workbook differs from the one on screen. */
  changed: boolean;
  /** Focus is inside the editor — someone is working in it. */
  busy: boolean;
}): "ignore" | "adopt" | "offer" {
  if (!now.changed) return "ignore";
  return now.busy ? "offer" : "adopt";
}

/** The read endpoint hands CSV back as text; a data: URL is base64 either way. */
export function decodeDataUrl(url: string): string {
  const b64 = url.split(",")[1] || "";
  try {
    return decodeURIComponent(escape(atob(b64)));
  } catch {
    return atob(b64);
  }
}

/**
 * RFC4180-ish: quote a field only when it would otherwise change meaning, and end
 * the file with a newline — a text file that doesn't is the "\ No newline at end of
 * file" every diff complains about, and this file is read by the agent and by other
 * tools, not only by us.
 */
export function toCsv(rows: unknown[][]): string {
  const cell = (v: unknown) => {
    const t = v == null ? "" : String(v);
    return /[",\n\r]/.test(t) ? `"${t.replace(/"/g, '""')}"` : t;
  };
  return rows.length ? rows.map((r) => r.map(cell).join(",")).join("\n") + "\n" : "";
}

/** Everything we need from a Univer teardown, without importing the types eagerly. */
export interface Instance {
  dispose: () => void;
}
