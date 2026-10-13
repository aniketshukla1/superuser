import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ClampedUserText } from "./Transcript";

afterEach(cleanup);

/** jsdom does no layout, so both heights are 0 and nothing ever overflows — which is
 *  exactly the "it fits" case. Overflow has to be staged. */
function withOverflow(run: () => void) {
  const d = (n: string, v: number) =>
    Object.defineProperty(HTMLElement.prototype, n, { configurable: true, value: v });
  d("scrollHeight", 240);
  d("clientHeight", 120);
  try {
    run();
  } finally {
    for (const n of ["scrollHeight", "clientHeight"]) {
      Object.defineProperty(HTMLElement.prototype, n, { configurable: true, value: 0 });
    }
  }
}

const box = () => document.querySelector(".user-clamp")!;

describe("the user prompt's fade", () => {
  // The regression: the mask lived on the base `.user-clamp` rule, so a two-line
  // question had its second line faded to half ink with nothing hidden behind it.
  // It read as disabled text. The fade is a promise that there is more below.
  it("is absent when the whole prompt fits", () => {
    render(<ClampedUserText text="Summarise the sheet for me." />);
    expect(box().className).not.toContain("is-clamped");
    expect(screen.queryByTestId("user-show-more")).toBeNull();
  });

  it("appears exactly when Show more does", () => {
    withOverflow(() => {
      render(<ClampedUserText text={"a line\n".repeat(40)} />);
      expect(box().className).toContain("is-clamped");
      expect(screen.getByTestId("user-show-more")).toBeTruthy();
    });
  });

  it("goes away once the prompt is open, since nothing is hidden then", () => {
    withOverflow(() => {
      render(<ClampedUserText text={"a line\n".repeat(40)} />);
      fireEvent.click(screen.getByTestId("user-show-more"));
      expect(box().className).toContain("is-open");
      expect(box().className).not.toContain("is-clamped");
      expect(screen.getByTestId("user-show-more").textContent).toBe("Show less");
    });
  });
});
