import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import { GeneratedView, HEIGHT_REPORTER, VIEW_HEIGHT, sealedDocument } from "./GeneratedView";
import { Transcript } from "./Transcript";
import type { Item } from "../types";

// What makes a model-written view safe is what its frame cannot do. These assert the
// frame's attributes and the document's policy, because both are load-bearing and both
// are one careless edit away from being undone.

afterEach(cleanup);

describe("a view the assistant drew", () => {
  it("runs in its own origin, with scripts but nothing else", () => {
    render(<GeneratedView title="Runway" html="<p>14 months</p>" />);
    const frame = screen.getByTestId("genview-frame") as HTMLIFrameElement;
    const sandbox = frame.getAttribute("sandbox") || "";
    expect(sandbox).toBe("allow-scripts");
    // The one combination that would undo the sandbox entirely.
    expect(sandbox).not.toContain("allow-same-origin");
    // A form in it submits nowhere; a link cannot move the app; it cannot open a window.
    for (const never of ["allow-forms", "allow-popups", "allow-top-navigation", "allow-modals"]) {
      expect(sandbox).not.toContain(never);
    }
    // The markup is inlined, never a URL something else could load.
    expect(frame.getAttribute("src")).toBeNull();
    expect(frame.getAttribute("srcdoc")).toContain("14 months");
  });

  it("grows to the height its own frame reports, within bounds, and ignores anyone else", () => {
    render(<GeneratedView title="Runway" html="<p>14 months</p>" />);
    const frame = screen.getByTestId("genview-frame") as HTMLIFrameElement;
    expect(frame.style.height).toBe(`${VIEW_HEIGHT.initial}px`);
    const say = (data: unknown, source: Window | null) =>
      act(() => {
        window.dispatchEvent(new MessageEvent("message", { data, source }));
      });
    say({ superuserViewHeight: 452.4 }, frame.contentWindow);
    expect(frame.style.height).toBe("453px");
    say({ superuserViewHeight: 99999 }, frame.contentWindow);
    expect(frame.style.height).toBe(`${VIEW_HEIGHT.max}px`);
    say({ superuserViewHeight: 3 }, frame.contentWindow);
    expect(frame.style.height).toBe(`${VIEW_HEIGHT.min}px`);
    // Not from this frame, or not a number: nothing changes.
    say({ superuserViewHeight: 500 }, window);
    say({ superuserViewHeight: "500" }, frame.contentWindow);
    say(null, frame.contentWindow);
    expect(frame.style.height).toBe(`${VIEW_HEIGHT.min}px`);
    // The document carries the line that reports it, after the model's markup.
    expect(frame.getAttribute("srcdoc")).toContain(`<p>14 months</p>${HEIGHT_REPORTER}</body>`);
  });

  it("the document forbids the network in the document itself", () => {
    const doc = sealedDocument("<p>hi</p>");
    expect(doc).toContain("default-src 'none'");
    expect(doc).toContain("form-action 'none'");
    expect(doc).toContain("base-uri 'none'");
    // Styles and inline script are the two things a drawn view needs.
    expect(doc).toContain("style-src 'unsafe-inline'");
    expect(doc).toContain("script-src 'unsafe-inline'");
  });

  it("says where it came from, so a transcript never has to be guessed at", () => {
    render(<GeneratedView title="Runway" html="<p>x</p>" source="budget.csv" />);
    expect(screen.getByText("drawn by the assistant · can’t send anything")).toBeTruthy();
    expect(screen.getByText("Source: budget.csv")).toBeTruthy();
  });

  it("is drawn in the transcript beside the charts, from the call that made it", () => {
    const items: Item[] = [
      { kind: "user", text: "compare the three plans" },
      { kind: "tool", id: "t1", name: "ui_component", args: { title: "Plans", html: "<b>Team</b>", source: "plans.csv" }, status: "ok" },
      { kind: "assistant", text: "Team is the one." },
    ];
    render(<Transcript items={items} onApprove={vi.fn()} />);
    expect(screen.getByTestId("transcript-view")).toBeTruthy();
    expect(screen.getByText("Plans")).toBeTruthy();
  });
});
