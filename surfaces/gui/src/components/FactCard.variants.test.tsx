import { describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach } from "vitest";
import { asCardSpec, FactCard } from "./FactCard";

// Three shapes a result kept being forced out of: a list of checks, a row of figures with
// how each moved, and one thing worth saying on its own. Same read-only card, same slots.

afterEach(cleanup);

describe("card variants", () => {
  it("a checklist says what passed and what did not", () => {
    const spec = asCardSpec({
      kind: "checklist",
      title: "Release checks",
      facts: [
        { label: "Tests", value: "1,458 passed", done: true },
        { label: "Secret scan", value: "not installed", done: false },
      ],
    });
    render(<FactCard spec={spec!} />);
    const items = screen.getByTestId("factcard-checks").querySelectorAll("li");
    expect(items.length).toBe(2);
    expect(items[0].getAttribute("data-done")).toBe("yes");
    expect(items[1].getAttribute("data-done")).toBe("no");
    expect(document.body.textContent).toContain("not installed");
  });

  it("metrics say how each figure moved, beside it", () => {
    const spec = asCardSpec({
      kind: "metrics",
      facts: [{ label: "Revenue", value: "£1,850", delta: "+12% on Q2", emphasis: true }],
    });
    render(<FactCard spec={spec!} />);
    expect(screen.getByText("+12% on Q2")).toBeTruthy();
    expect(screen.getByText("£1,850")).toBeTruthy();
  });

  it("a notice is its sentence, and needs no facts", () => {
    const spec = asCardSpec({ kind: "notice", tone: "warn", title: "Heads up", note: "The deploy freeze is still on." });
    expect(spec).toBeTruthy();
    render(<FactCard spec={spec!} />);
    expect(screen.getByText("The deploy freeze is still on.")).toBeTruthy();
    expect(screen.getByTestId("factcard").getAttribute("data-tone")).toBe("warn");
    // A notice with nothing to say is not a card.
    expect(asCardSpec({ kind: "notice", title: "Heads up" })).toBeNull();
  });

  it("an unknown kind falls back to facts rather than drawing nothing", () => {
    const spec = asCardSpec({ kind: "dashboard", facts: [{ label: "Runway", value: "14 months" }] });
    expect(spec?.kind).toBe("facts");
    render(<FactCard spec={spec!} />);
    expect(screen.getByText("14 months")).toBeTruthy();
  });
});
