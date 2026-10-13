import { describe, expect, it } from "vitest";
import { pageTitle } from "./KnowledgeSection";

describe("pageTitle", () => {
  it("shows a slug as words", () => {
    expect(pageTitle("status-note")).toBe("Status note");
    expect(pageTitle("release_checklist")).toBe("Release checklist");
  });

  it("drops a filename's extension", () => {
    expect(pageTitle("brief.md")).toBe("Brief");
  });

  it("leaves a name someone chose as chosen", () => {
    expect(pageTitle("Status Brief")).toBe("Status Brief");
    expect(pageTitle("Postgres")).toBe("Postgres");
    expect(pageTitle("my notes on k8s")).toBe("my notes on k8s");
  });
});
