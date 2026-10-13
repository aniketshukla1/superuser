import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { Transcript } from "./Transcript";
import { ApprovalCard } from "./ApprovalCard";
import { itemsFromMessages } from "../itemsFromMessages";
import { humanizeApprovalTitle, humanizeTool } from "../humanize";
import type { Item } from "../types";

// The assistant's browser in the conversation: a kept screen is drawn outside the collapsed
// steps (like a chart, and for the same reason), a request for the user's hands never splits
// the turn, a reopened conversation keeps the screens in the order they were taken, and an
// approval for a click reads as the control and the site — never as a ref.

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const stubFrames = () =>
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => ({ ok: true, json: async () => ({ ok: true, data_url: "data:image/jpeg;base64,AAAA" }) }) as Response),
  );

const shot: Item = { kind: "computer", persona: "assistant", frame: "f-1-0001", url: "https://example.com/a", title: "Example" };

describe("kept screens in the transcript", () => {
  it("draws the screen outside the collapsed steps, before the answer", () => {
    stubFrames();
    const items: Item[] = [
      { kind: "user", text: "check example.com" },
      { kind: "tool", id: "t1", name: "browser_navigate", args: { url: "https://example.com/a" }, status: "ok" },
      shot,
      { kind: "assistant", text: "It says hello." },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    const tile = screen.getByTestId("transcript-computer");
    const answer = screen.getByText("It says hello.");
    expect(tile.compareDocumentPosition(answer) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(container.querySelectorAll(".stepgroup").length).toBe(1);
  });

  it("a request for the user's hands does not split the turn", () => {
    const items: Item[] = [
      { kind: "user", text: "log in and read my inbox" },
      { kind: "tool", id: "t1", name: "browser_navigate", args: { url: "https://mail.example.com" }, status: "ok" },
      { kind: "assistant", text: "It wants a sign-in." },
      { kind: "computer_help", persona: "assistant", reason: "Sign in", url: "https://mail.example.com", resolved: "taken" },
      { kind: "tool", id: "t2", name: "browser_read", args: {}, status: "ok" },
      { kind: "assistant", text: "You have three new messages." },
    ];
    const { container } = render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(container.querySelectorAll(".stepgroup").length).toBe(1);
    expect(screen.queryByTestId("computer-help-card")).toBeNull();
  });

  it("a reopened conversation keeps screens and charts in the order they were made", () => {
    const items = itemsFromMessages([
      {
        role: "assistant",
        content: "Done.",
        items: [
          { kind: "computer", persona: "assistant", frame: "f-1-0001", url: "https://a.example/", title: "A" },
          { kind: "tool", name: "chart", args: { spec: {} }, status: "ok" },
          { kind: "computer", persona: "assistant", frame: "f-2-0002", url: "https://b.example/", title: "B" },
        ],
      },
    ]);
    expect(items.map((it) => (it.kind === "computer" ? it.frame : it.kind))).toEqual(["f-1-0001", "tool", "f-2-0002", "assistant"]);
  });
});

describe("browser steps and approvals in words", () => {
  it("names what happened, not the model's handles", () => {
    expect(humanizeTool("browser_type", { ref: "e4", snapshot_id: 2, text: "running shoes", submit: true })).toEqual({
      pre: "Typed ",
      obj: "“running shoes”",
      post: " and pressed Enter",
    });
    expect(humanizeTool("browser_navigate", { url: "https://shop.example/cart" }).obj).toBe("shop.example");
    expect(humanizeApprovalTitle("browser_click", { click: "“Place order”", site: "shop.example" })).toEqual({
      pre: "Click ",
      obj: "“Place order”",
      post: " on shop.example",
    });
    expect(humanizeApprovalTitle("browser_type", { into: "“Search”", site: "shop.example", then: "press Enter" }).post).toBe(
      " on shop.example and press Enter",
    );
  });

  it("an approval card for a click says where it acts and what goes in", () => {
    render(
      <ApprovalCard
        item={{
          kind: "approval",
          name: "browser_type",
          args: { into: "“Search”", text: "running shoes", site: "shop.example", then: "" },
          reason: "It read a page this turn, so acting on shop.example is confirmed first.",
          category: "browser",
        }}
        onApprove={vi.fn()}
      />,
    );
    expect(screen.getByText("acts on shop.example in its browser")).toBeTruthy();
    expect(screen.getByText("Always allow on shop.example")).toBeTruthy();
    expect(document.body.textContent).toContain("running shoes");
    expect(document.body.textContent).not.toMatch(/ref=|snapshot_id/);
  });
});
