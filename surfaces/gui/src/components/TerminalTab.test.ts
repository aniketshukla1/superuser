// The terminal's fit loop is a layout bug, not a logic bug, so it is guarded here as
// a layout invariant. jsdom has no layout engine, so these read the stylesheet the
// app actually ships rather than a rendered box.
//
// The bug being guarded (fixed 2026-08-19): `.terminal-host` carried `padding: 8px`.
// The xterm fit addon sizes the grid from `getComputedStyle(host).height/width`, and
// under the app's global `box-sizing: border-box` that returns the BORDER box — the
// padding included. So the addon believed it had 16px more room than it did in each
// axis, sized the grid to match, and xterm spilled past its host. The spill raised a
// scrollbar on the enclosing `.wp-pane` (`overflow: auto`), the scrollbar took 15px
// back, the ResizeObserver saw the host change and refit — and around it went. Every
// lap cleared the renderer and SIGWINCHed the shell, which is what the flicker and
// the endlessly redrawn prompt actually were.
import { describe, expect, it } from "vitest";
import { readStyles } from "../testing/source";

const css = readStyles();

/** The declaration block of the first rule whose selector list matches exactly. */
function block(selector: string): string {
  const at = css.indexOf(selector + " {");
  expect(at, `no rule for \`${selector}\``).toBeGreaterThan(-1);
  return css.slice(css.indexOf("{", at) + 1, css.indexOf("}", at));
}

describe("terminal layout", () => {
  it("leaves the measured host unpadded, so the fit addon reads real space", () => {
    // Any padding here is silently added to what the addon thinks it has.
    expect(block(".terminal-host")).not.toMatch(/(^|;|\s)padding\s*:/);
  });

  it("keeps the inset, just one box further out where nothing measures it", () => {
    // --s-4 is 8px; the spacing sweep moved this from the literal. Both are asserted so
    // the inset cannot quietly change size by the scale moving underneath it.
    expect(block(".terminal-tab")).toMatch(/padding\s*:\s*var\(--s-4\)/);
    expect(css).toMatch(/--s-4:\s*8px/);
  });

  it("clips the host, so a mis-fit can never reach the pane's scrollbar", () => {
    expect(block(".terminal-host")).toMatch(/overflow\s*:\s*hidden/);
  });

  it("does not let the terminal's pane scroll — xterm scrolls itself", () => {
    expect(block(".wp-pane.wp-pane-fill")).toMatch(/overflow\s*:\s*hidden/);
  });
});
