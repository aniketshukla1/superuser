import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import { asCardSpec, FactCard, type CardSpec } from "./FactCard";
import { Transcript } from "./Transcript";
import type { Item } from "../types";

afterEach(cleanup);

const SPEC: CardSpec = {
  title: "Runway",
  tone: "warn",
  note: "Assumes the two hires land in March.",
  facts: [
    { label: "Cash", value: "£1.24m" },
    { label: "Monthly burn", value: "£96k" },
    { label: "Runs out", value: "March 2027", emphasis: true },
  ],
};

describe("a card spec", () => {
  it("drops half-facts and refuses a card with nothing left", () => {
    expect(asCardSpec(null)).toBeNull();
    expect(asCardSpec({ facts: [] })).toBeNull();
    // A label with no value says nothing; a value with no label is a number with no
    // meaning, which is the failure a card exists to avoid.
    expect(asCardSpec({ facts: [{ label: "", value: "12" }] })).toBeNull();
    expect(asCardSpec({ facts: [{ label: "Cash" }] })).toBeNull();
    // A good fact beside a broken one survives on its own.
    const kept = asCardSpec({ facts: [{ label: "", value: "1" }, { label: "Cash", value: "2" }] });
    expect(kept?.facts.map((f) => f.label)).toEqual(["Cash"]);
  });

  it("falls back to a neutral tone rather than trusting an unknown one", () => {
    // The tone reaches CSS as a data attribute. An unchecked value would style
    // nothing and leave the card unbordered, which reads as a rendering bug.
    expect(asCardSpec({ facts: SPEC.facts, tone: "catastrophic" } as never)?.tone).toBe("neutral");
    expect(asCardSpec({ facts: SPEC.facts, tone: "bad" } as never)?.tone).toBe("bad");
  });
});

describe("a card in the conversation", () => {
  it("shows every fact with its label, and marks the one it is about", () => {
    render(<FactCard spec={SPEC} />);
    const card = screen.getByTestId("factcard");
    expect(card.getAttribute("data-tone")).toBe("warn");
    for (const f of SPEC.facts) {
      expect(within(card).getByText(f.label)).toBeTruthy();
      expect(within(card).getByText(f.value)).toBeTruthy();
    }
    expect(card.querySelectorAll(".factcard-fact.is-lead")).toHaveLength(1);
    expect(within(card).getByText("Assumes the two hires land in March.")).toBeTruthy();
  });

  it("has nothing that makes anything happen", () => {
    // The design line. A summary block that could act would be a new authority
    // hiding inside a read-only shape — the agent already has reviewed ways to act.
    render(<FactCard spec={{ ...SPEC, source: "ledger.xlsx" }} />);
    const card = screen.getByTestId("factcard");
    expect(card.querySelectorAll("button, a, input, form")).toHaveLength(0);
    expect(within(card).getByText(/ledger\.xlsx/)).toBeTruthy();
  });
});

describe("cards in a turn", () => {
  it("renders one the agent composed, and ignores a refused one", () => {
    const ok: Item[] = [
      { kind: "user", text: "how long do we have" },
      { kind: "tool", id: "c1", name: "card", args: { spec: SPEC, source: "ledger.xlsx" }, status: "ok" },
      { kind: "assistant", text: "March, on current burn." },
    ];
    render(<Transcript items={ok} onApprove={vi.fn()} />);
    expect(screen.getByTestId("transcript-card")).toBeTruthy();
    cleanup();

    // A refused call carries a spec too, and drawing it would state exactly the
    // figures the gate just stopped as if they were established.
    const refused: Item[] = [
      { kind: "user", text: "x" },
      { kind: "tool", id: "c1", name: "card", args: { spec: SPEC }, status: "error" },
    ];
    render(<Transcript items={refused} onApprove={vi.fn()} />);
    expect(screen.queryByTestId("transcript-card")).toBeNull();
  });

  it("keeps a chart, a table and a card in the order the turn produced them", () => {
    // One ordered pass over the steps. Collecting per kind would group them and
    // quietly rearrange the argument the turn was making.
    const items: Item[] = [
      { kind: "user", text: "walk me through it" },
      { kind: "tool", id: "1", name: "card", args: { spec: SPEC, source: "s" }, status: "ok" },
      {
        kind: "tool",
        id: "2",
        name: "chart",
        args: {
          spec: { type: "bar", x: { field: "a" }, y: { field: "b" }, data: [{ a: "x", b: 1 }] },
          source: "s",
        },
        status: "ok",
      },
      {
        kind: "tool",
        id: "3",
        name: "table",
        args: { spec: { columns: [{ field: "a" }], rows: [{ a: 1 }] }, source: "s" },
        status: "ok",
      },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    const order = [...container.querySelectorAll("[data-testid^='transcript-']")].map((n) =>
      n.getAttribute("data-testid"),
    );
    expect(order).toEqual(["transcript-card", "transcript-chart", "transcript-table"]);
  });
});
