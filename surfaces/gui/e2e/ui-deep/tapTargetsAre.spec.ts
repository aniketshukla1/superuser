import { test, expect } from "../fixtures";
import type { Page } from "@playwright/test";

/** Wait until the app has actually painted, not just responded — the sidebar's Scheduled
 *  band included. It arrives on its own fetch after the account row, so a probe that ran
 *  first audited a sidebar without it and passed or failed on timing (its header button
 *  sat at 16px under the target floor, found only when the band won the race). */
async function ready(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("account-row")).toBeVisible();
  await expect(page.getByTestId("scheduled-band")).toBeVisible();
}
// ----------------------------------------------------------- hit size ---

test("tap targets are big enough to hit", async ({ page }) => {
  await ready(page);
  // WCAG 2.2 AA (2.5.8) sets 24×24 CSS px as the floor for pointer targets.
  const small = await page.evaluate(() => {
    const out: string[] = [];
    for (const el of Array.from(document.querySelectorAll<HTMLElement>("button, a[href], [role=button]"))) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      if (r.width < 24 || r.height < 24) {
        const name = (el.getAttribute("aria-label") || el.textContent || "").trim().slice(0, 30);
        out.push(`"${name}" ${Math.round(r.width)}×${Math.round(r.height)}`);
      }
    }
    return out;
  });
  expect(small, `targets below the 24×24 floor:\n  ${small.join("\n  ")}`).toEqual([]);
});

// ------------------------------------------------------ reduced motion ---

test("reduced motion is respected", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await ready(page);
  const moving = await page.evaluate(() => {
    const out: string[] = [];
    for (const el of Array.from(document.querySelectorAll<HTMLElement>("body *"))) {
      if (el.getBoundingClientRect().height === 0) continue;
      const s = getComputedStyle(el);
      const dur = parseFloat(s.animationDuration) || 0;
      const inf = s.animationIterationCount === "infinite";
      if (inf && dur > 0) out.push(`${el.tagName.toLowerCase()}.${String(el.className).slice(0, 30)} ${s.animationName}`);
    }
    return out;
  });
  expect(moving, `still animating infinitely under prefers-reduced-motion:\n  ${moving.join("\n  ")}`).toEqual([]);
});
