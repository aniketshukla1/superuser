import { describe, expect, it } from "vitest";
import { plainText } from "./plainText";

describe("plainText", () => {
  it("drops emphasis, headings and list markers but keeps the words", () => {
    // Verbatim from a Morning brief run preview.
    const md = "### Project Overview\n**Superuser** is a *personal* assistant.\n*   **Status:** Done\n1. First";
    expect(plainText(md)).toBe("Project Overview\nSuperuser is a personal assistant.\nStatus: Done\nFirst");
  });

  it("keeps link text and code, not their syntax", () => {
    expect(plainText("See [the plan](https://x.test) and `brief.md`.")).toBe("See the plan and brief.md.");
    expect(plainText("```ts\nconst a = 1;\n```")).toBe("const a = 1;");
  });

  it("leaves snake_case and arithmetic alone", () => {
    expect(plainText("run find_tools with 2 * 3")).toBe("run find_tools with 2 * 3");
  });
});
