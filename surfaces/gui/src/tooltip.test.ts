import { describe, expect, it } from "vitest";
import { tipTarget } from "./tooltip";

describe("tipTarget", () => {
  const make = (html: string) => {
    document.body.innerHTML = html;
    return document.body.firstElementChild as HTMLElement;
  };

  it("finds the control with a data-tip from anywhere inside it", () => {
    const b = make('<button data-tip="Settings" aria-label="Settings"><svg></svg></button>');
    expect(tipTarget(b.querySelector("svg"))).toBe(b);
  });

  // Rows are found by their title (by people's tools and by the e2e suite), so a title is
  // the browser's to show and is never rewritten into a data-tip.
  it("leaves a plain title alone", () => {
    const b = make('<button title="Weekly plan 6">Weekly plan 6</button>');
    expect(tipTarget(b)).toBeNull();
    expect(b.getAttribute("title")).toBe("Weekly plan 6");
  });
});
