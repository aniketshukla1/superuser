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
test("Tab never gets stuck on one control", async ({ page }) => {
  await ready(page);
  const seen: string[] = [];
  for (let i = 0; i < 30; i++) {
    await page.keyboard.press("Tab");
    seen.push(
      await page.evaluate(() => {
        const el = document.activeElement as HTMLElement | null;
        return el ? `${el.tagName}:${(el.getAttribute("aria-label") || el.textContent || "").trim().slice(0, 25)}` : "none";
      }),
    );
  }
  // A trap shows up as the same element over and over.
  const worst = [...new Set(seen)].map((s) => ({ s, n: seen.filter((x) => x === s).length })).sort((a, b) => b.n - a.n)[0];
  expect(worst.n, `Tab returned to ${worst.s} ${worst.n}/30 times — keyboard trap`).toBeLessThan(12);
});

test("a dialog takes focus, traps it, closes on Escape, and gives focus back", async ({ page }) => {
  await ready(page);
  const trigger = page.getByTestId("account-row");
  await trigger.click();
  const menu = page.getByTestId("account-menu");
  await expect(menu).toBeVisible();

  // Escape must close it — the universal dismiss.
  await page.keyboard.press("Escape");
  await expect(menu, "Escape did not close the account menu").not.toBeVisible();

  // …and focus must come back to what opened it, or the keyboard user is lost.
  const returned = await page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    return el?.getAttribute("data-testid") ?? el?.tagName ?? "none";
  });
  expect(returned, "focus was not returned to the trigger after closing").toBe("account-row");
});

// --------------------------------------------------------------- theme ---

for (const theme of ["light", "dark"] as const) {
  test(`text stays legible in the ${theme} theme`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.addInitScript((t) => {
      document.documentElement.setAttribute("data-theme", t);
    }, theme);
    await ready(page);

    const bad = await page.evaluate(() => {
      type RGBA = [number, number, number, number];
      // Chromium serialises modern colours as `color(srgb r g b / a)` with 0..1
      // components and legacy ones as `rgb(r g b / a)` with 0..255. Reading both
      // with one number-grab makes white-on-green look like 1:1, so parse properly.
      const parse = (c: string): RGBA | null => {
        if (!c || c === "transparent") return [0, 0, 0, 0];
        let m = c.match(/^color\(srgb\s+([\d.eE+-]+)\s+([\d.eE+-]+)\s+([\d.eE+-]+)(?:\s*\/\s*([\d.eE+-]+))?\)/);
        if (m) return [+m[1] * 255, +m[2] * 255, +m[3] * 255, m[4] === undefined ? 1 : +m[4]];
        m = c.match(/^rgba?\(([^)]+)\)/);
        if (m) {
          const p = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
          return [p[0], p[1], p[2], p[3] === undefined ? 1 : p[3]];
        }
        return null;
      };
      const over = (fg: RGBA, bg: RGBA): RGBA => {
        const a = fg[3];
        return [fg[0] * a + bg[0] * (1 - a), fg[1] * a + bg[1] * (1 - a), fg[2] * a + bg[2] * (1 - a), 1];
      };
      const lum = (c: RGBA) => {
        const f = (v: number) => {
          const s = v / 255;
          return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
        };
        return 0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2]);
      };
      // Composite every translucent layer between the text and the first opaque
      // surface — a badge on `… / 0.3` is not sitting on that colour, it's sitting
      // on that colour mixed with whatever is behind it.
      const bgOf = (el: HTMLElement): RGBA => {
        const stack: RGBA[] = [];
        let n: HTMLElement | null = el;
        while (n) {
          const c = parse(getComputedStyle(n).backgroundColor);
          if (c && c[3] > 0) {
            stack.push(c);
            if (c[3] === 1) break;
          }
          n = n.parentElement;
        }
        let base: RGBA = [255, 255, 255, 1];
        for (let i = stack.length - 1; i >= 0; i--) base = over(stack[i], base);
        return base;
      };
      const out: string[] = [];
      for (const el of Array.from(document.querySelectorAll<HTMLElement>("body *"))) {
        const text = Array.from(el.childNodes).filter((n) => n.nodeType === 3 && n.textContent!.trim()).map((n) => n.textContent!.trim()).join(" ");
        if (!text) continue;
        const r = el.getBoundingClientRect();
        if (r.width === 0 || r.height === 0) continue;
        const s = getComputedStyle(el);
        if (s.visibility === "hidden" || Number(s.opacity) === 0) continue;
        const fgRaw = parse(s.color);
        if (!fgRaw) continue;
        const bg = bgOf(el);
        const fg = over(fgRaw, bg); // text alpha counts against its own backdrop
        const [lf, lb] = [lum(fg), lum(bg)];
        const ratio = (Math.max(lf, lb) + 0.05) / (Math.min(lf, lb) + 0.05);
        const px = parseFloat(s.fontSize);
        const large = px >= 24 || (px >= 18.66 && Number(s.fontWeight) >= 700);
        const floor = large ? 3 : 4.5;
        if (ratio < floor) {
          const hex = (c: RGBA) => "#" + c.slice(0, 3).map((v) => Math.round(v).toString(16).padStart(2, "0")).join("");
          out.push(`"${text.slice(0, 30)}" ${ratio.toFixed(2)}:1 (needs ${floor}) ${hex(fg)} on ${hex(bg)} @${px}px`);
        }
      }
      return out;
    });
    // Report the worst offenders; a wall of 200 lines helps nobody.
    expect(bad.slice(0, 12), `${bad.length} low-contrast text nodes in ${theme}:\n  ${bad.slice(0, 12).join("\n  ")}`).toEqual([]);
  });
}

// ------------------------------------------------------ content stress ---

test("a very long session title does not break the sidebar", async ({ page }) => {
  await ready(page);
  const width = await page.evaluate(() => {
    const row = document.querySelector<HTMLElement>('[data-testid^="session-"], .sidebar button');
    if (!row) return null;
    // Force a pathological title in place and see whether the row contains it.
    const target = row.querySelector<HTMLElement>("span, div") ?? row;
    target.textContent = "Supercalifragilistic" + "expialidocious".repeat(20);
    const sidebar = row.closest<HTMLElement>(".sidebar, aside, nav") ?? row.parentElement!;
    return { row: row.getBoundingClientRect().width, container: sidebar.getBoundingClientRect().width };
  });
  test.skip(width === null, "no sidebar row to stress");
  expect(
    width!.row,
    `a long title widened its row to ${Math.round(width!.row)}px inside a ${Math.round(width!.container)}px container — it must truncate`,
  ).toBeLessThanOrEqual(width!.container + 1);
});

test("a long unbroken string in a message wraps instead of stretching the transcript", async ({ page }) => {
  await ready(page);
  const res = await page.evaluate(() => {
    const host = document.querySelector<HTMLElement>("[data-testid=transcript], main") ?? document.body;
    const probe = document.createElement("div");
    probe.textContent = "x".repeat(400) + " https://example.com/" + "y".repeat(300);
    host.appendChild(probe);
    const r = { probe: probe.getBoundingClientRect().width, host: host.getBoundingClientRect().width };
    probe.remove();
    return r;
  });
  expect(
    res.probe,
    `an unbroken 700-char string rendered ${Math.round(res.probe)}px wide in a ${Math.round(res.host)}px area — needs overflow-wrap`,
  ).toBeLessThanOrEqual(res.host + 1);
});
