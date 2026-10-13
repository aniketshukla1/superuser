// Dropping a spreadsheet: what happens, and when it does not.
//
// The rule these functions encode is that a drop with an EMPTY composer is an instruction —
// "look at this" — and a drop with something typed is an attachment to the thought already
// in progress. Hijacking the second case would take a half-written question and send
// something else instead, which is a worse failure than making someone press Enter.

/** Extensions worth analysing on sight. Deliberately narrow: these are tables, and the
 *  analysis prompt below only makes sense for a table. A dropped .png or .pdf attaches the
 *  way it always has. */
const DATA_EXT = ["csv", "tsv", "xlsx", "xlsm", "xls", "ods"];

export function isDataFile(name: string): boolean {
  const ext = (name.split(".").pop() || "").toLowerCase();
  return DATA_EXT.includes(ext);
}

/** Should this drop start an analysis by itself? */
export function shouldAutoAnalyse(opts: {
  names: string[];
  /** What is already typed. Anything at all means the drop is an attachment. */
  draft: string;
  /** A turn in flight owns the conversation; a second one would interleave. */
  running: boolean;
}): boolean {
  return !opts.running && opts.draft.trim() === "" && opts.names.some(isDataFile);
}

/** The turn a dropped table starts.
 *
 * Names the file by PATH because it is now on disk in the session's folder — that is what
 * makes `sheet_profile` and `chart` able to reach it, and the difference between this and
 * pasting the contents into the prompt.
 *
 * It asks for the profile FIRST and says charts come from real values, because the failure
 * mode on a small model is not refusing to answer — it is inventing plausible numbers and
 * presenting them as findings. */
export function analysePrompt(names: string[]): string {
  const files = names.filter(isDataFile);
  const list = files.map((n) => `\`${n}\``).join(", ");
  const one = files.length === 1;
  return (
    `I've put ${list} in this workspace. ` +
    `Use sheet_profile on ${one ? "it" : "each one"} to see what the columns are, then read the rows you need. ` +
    `Draw the two or three charts that actually say something about this data — using the real values you read, never remembered or rounded ones — ` +
    `and then tell me in a few sentences what stands out: the shape of it, anything surprising, and anything that looks wrong or missing.`
  );
}
