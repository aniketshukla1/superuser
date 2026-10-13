import { describe, expect, it } from "vitest";
import { analysePrompt, isDataFile, shouldAutoAnalyse } from "./dropAnalyse";

describe("dropping a file", () => {
  it("treats tables as tables and leaves everything else alone", () => {
    for (const n of ["sales.csv", "Q3.XLSX", "data.tsv", "book.ods", "old.xls"]) {
      expect(isDataFile(n), n).toBe(true);
    }
    for (const n of ["diagram.png", "notes.md", "report.pdf", "script.py", "archive.zip", "noextension"]) {
      expect(isDataFile(n), n).toBe(false);
    }
  });

  it("starts an analysis when a table lands on an empty composer", () => {
    expect(shouldAutoAnalyse({ names: ["sales.csv"], draft: "", running: false })).toBe(true);
    expect(shouldAutoAnalyse({ names: ["a.png", "sales.xlsx"], draft: "", running: false })).toBe(true);
  });

  // The one that matters. Someone half way through typing a question drops a file to go with
  // it; sending an analysis instead would throw their words away and answer a different
  // question. Whitespace is not a draft, but anything else is.
  it("never takes over a composer someone is already using", () => {
    expect(shouldAutoAnalyse({ names: ["sales.csv"], draft: "what were Q3 margins", running: false })).toBe(false);
    expect(shouldAutoAnalyse({ names: ["sales.csv"], draft: "   \n ", running: false })).toBe(true);
  });

  // A turn in flight owns the conversation; a second one would interleave with it.
  it("waits while a turn is running", () => {
    expect(shouldAutoAnalyse({ names: ["sales.csv"], draft: "", running: true })).toBe(false);
  });

  it("does nothing for files that are not tables", () => {
    expect(shouldAutoAnalyse({ names: ["diagram.png", "notes.md"], draft: "", running: false })).toBe(false);
    expect(shouldAutoAnalyse({ names: [], draft: "", running: false })).toBe(false);
  });

  // The prompt names the file by PATH — that is what lets sheet_profile and chart reach it,
  // and the difference between this and pasting the contents into the message.
  it("asks for the profile first, and for real numbers", () => {
    const p = analysePrompt(["sales.csv", "notes.md"]);
    expect(p).toContain("`sales.csv`");
    expect(p).not.toContain("notes.md");
    expect(p).toContain("sheet_profile");
    expect(p.toLowerCase()).toContain("real values");
  });

  it("reads naturally for one file and for several", () => {
    expect(analysePrompt(["a.csv"])).toContain("on it");
    expect(analysePrompt(["a.csv", "b.csv"])).toContain("on each one");
  });
});
