import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { CHAT_MIN_W, fitBeside, usePanelWidth } from "./ResizeHandle";

function setWindowWidth(px: number) {
  Object.defineProperty(window, "innerWidth", { value: px, configurable: true, writable: true });
}

describe("panel widths", () => {
  beforeEach(() => {
    localStorage.clear();
    setWindowWidth(1440);
  });

  it("starts at the fallback with nothing stored", () => {
    const { result } = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    expect(result.current[0]).toBe(560);
  });

  it("remembers a width across mounts", () => {
    const first = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    act(() => first.result.current[1](800));
    const second = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    expect(second.result.current[0]).toBe(800);
  });

  it("refuses to shrink a panel below the width that keeps it usable", () => {
    const { result } = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    act(() => result.current[1](50));
    expect(result.current[0]).toBe(360);
  });

  it("refuses to widen a panel past its share of the window", () => {
    const { result } = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    act(() => result.current[1](5000));
    expect(result.current[0]).toBe(Math.round(1440 * 0.72));
  });

  it("re-clamps to the new window when it shrinks", () => {
    // The bug this guards: bounds captured at mount went stale on resize, and a panel
    // dragged wide on a large display came back narrower than its own minimum.
    const { result } = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    act(() => result.current[1](900));
    expect(result.current[0]).toBe(900);

    act(() => {
      setWindowWidth(800);
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current[0]).toBe(Math.round(800 * 0.72)); // 576, not 900
  });

  it("never reports a width below the minimum, however small the window gets", () => {
    const { result } = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    act(() => {
      setWindowWidth(320);
      window.dispatchEvent(new Event("resize"));
    });
    expect(result.current[0]).toBeGreaterThanOrEqual(360);
  });

  it("survives a corrupt stored value rather than laying out at NaN", () => {
    localStorage.setItem("k", "not-a-number");
    const { result } = renderHook(() => usePanelWidth("k", { fallback: 560, min: 360, maxFraction: 0.72 }));
    expect(result.current[0]).toBe(560);
  });
});

// The reported bug: drag the right panel and the conversation keeps giving until there is
// nothing left of it. Both panels were inside their own limits the whole time — the
// limits just never mentioned each other.
describe("leaving room for the conversation", () => {
  const PANEL = { fallback: 560, min: 360, maxFraction: 0.72 };

  it("caps a panel so the reserved space survives", () => {
    setWindowWidth(1440);
    const { result } = renderHook(() =>
      usePanelWidth("k", { ...PANEL, reserve: CHAT_MIN_W + 240 }),
    );
    act(() => result.current[1](1300));
    expect(result.current[0]).toBe(1440 - 240 - CHAT_MIN_W);
  });

  it("still honours the fraction when that is the tighter of the two", () => {
    setWindowWidth(3000);
    const { result } = renderHook(() => usePanelWidth("k", { ...PANEL, reserve: CHAT_MIN_W }));
    act(() => result.current[1](2500));
    // 0.72 of 3000 is 2160; the reserve would allow 2580. The lower ceiling wins.
    expect(result.current[0]).toBe(2160);
  });

  // A window too small for all three is a window problem. Making the panel unusable too
  // would not give the conversation a usable width either.
  it("stops at the panel's own minimum rather than collapsing it", () => {
    setWindowWidth(880);
    const { result } = renderHook(() =>
      usePanelWidth("k", { ...PANEL, reserve: CHAT_MIN_W + 224 }),
    );
    act(() => result.current[1](900));
    expect(result.current[0]).toBe(360);
  });

  it("reserving nothing is the old behaviour", () => {
    setWindowWidth(1440);
    const { result } = renderHook(() => usePanelWidth("k", PANEL));
    act(() => result.current[1](2000));
    expect(result.current[0]).toBe(Math.round(1440 * 0.72));
  });
});

describe("fitting two panels beside each other", () => {
  it("squeezes the panel so the conversation keeps its minimum", () => {
    expect(fitBeside(900, { winW: 1440, other: 240, min: 360 })).toBe(1440 - 240 - CHAT_MIN_W);
  });

  it("leaves a panel alone when it already fits", () => {
    expect(fitBeside(500, { winW: 1440, other: 240, min: 360 })).toBe(500);
  });

  it("gives the panel back its full width when the other one closes", () => {
    expect(fitBeside(900, { winW: 1440, other: 0, min: 360 })).toBe(900);
  });

  it("never goes under the panel's own minimum", () => {
    expect(fitBeside(900, { winW: 880, other: 224, min: 360 })).toBe(360);
  });

  // The reason this is a render-time clamp and not a commit-time one: a width set on a
  // large display should come back when the window is large again, not be thrown away
  // the first time it does not fit.
  it("does not destroy the stored preference — it only renders it smaller", () => {
    const stored = 900;
    // 1200 - 240 - 420 = 540, comfortably above the panel's own floor, so this is the
    // squeeze and not the floor. (At 1000 the arithmetic gives 340 and the floor wins,
    // which is a different behaviour and covered by the test above.)
    expect(fitBeside(stored, { winW: 1200, other: 240, min: 360 })).toBe(540);
    expect(fitBeside(stored, { winW: 1920, other: 240, min: 360 })).toBe(stored);
  });
});
