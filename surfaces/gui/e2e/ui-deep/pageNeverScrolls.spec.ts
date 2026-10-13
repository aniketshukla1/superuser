import { test, expect } from "../fixtures";
import type { Page } from "@playwright/test";

const WIDTHS = [
  { w: 1440, h: 900, label: "desktop" },
  { w: 1024, h: 768, label: "small laptop" },
  { w: 768, h: 1024, label: "tablet" },
];

/** Wait until the app has actually painted, not just responded — the sidebar's Scheduled
 *  band included. It arrives on its own fetch after the account row, so a probe that ran
 *  first audited a sidebar without it and passed or failed on timing (its header button
 *  sat at 16px under the target floor, found only when the band won the race). */
async function ready(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("account-row")).toBeVisible();
  await expect(page.getByTestId("scheduled-band")).toBeVisible();
}
// ------------------------------------------------------------ layout ---

test("the page never scrolls sideways, at any width", async ({ page }) => {
  // Horizontal overflow on an app shell is always a bug: it means something is
  // wider than its container and the user has to scroll to reach it.
  const bad: string[] = [];
  for (const { w, h, label } of WIDTHS) {
    await page.setViewportSize({ width: w, height: h });
    await ready(page);
    const over = await page.evaluate(() => {
      const d = document.documentElement;
      if (d.scrollWidth <= d.clientWidth) return null;
      // Name the widest offender so the failure is actionable.
      let worst = { sel: "", right: 0 };
      for (const el of Array.from(document.querySelectorAll<HTMLElement>("body *"))) {
        const r = el.getBoundingClientRect();
        if (r.width === 0 || r.height === 0) continue;
        if (r.right > worst.right) {
          worst = {
            sel: `${el.tagName.toLowerCase()}${el.className && typeof el.className === "string" ? "." + el.className.trim().split(/\s+/).slice(0, 2).join(".") : ""}`,
            right: Math.round(r.right),
          };
        }
      }
      return { scrollWidth: d.scrollWidth, clientWidth: d.clientWidth, worst };
    });
    if (over) bad.push(`${label} (${w}px): scrollWidth ${over.scrollWidth} > ${over.clientWidth}; widest = ${over.worst.sel} @ ${over.worst.right}px`);
  }
  expect(bad, `horizontal overflow:\n  ${bad.join("\n  ")}`).toEqual([]);
});

test("nothing important is clipped or pushed off-screen at a small laptop width", async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 768 });
  await ready(page);
  // The composer is the primary action; if it's off-screen the app is unusable.
  const composer = page.getByTestId("composer-input");
  await expect(composer).toBeVisible();
  const box = await composer.boundingBox();
  expect(box, "composer has no box").not.toBeNull();
  expect(box!.x, "composer starts off the left edge").toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width, "composer runs past the right edge").toBeLessThanOrEqual(1024 + 1);
});

// ------------------------------------------------------ accessibility ---

test("every interactive control has an accessible name", async ({ page }) => {
  await ready(page);
  const nameless = await page.evaluate(() => {
    const out: string[] = [];
    const sel = "button, a[href], input, select, textarea, [role=button], [role=tab], [role=menuitem]";
    for (const el of Array.from(document.querySelectorAll<HTMLElement>(sel))) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue; // not rendered
      if (el.getAttribute("aria-hidden") === "true") continue;
      const name =
        el.getAttribute("aria-label") ||
        el.getAttribute("title") ||
        (el.getAttribute("aria-labelledby") &&
          document.getElementById(el.getAttribute("aria-labelledby")!)?.textContent) ||
        (el as HTMLInputElement).labels?.[0]?.textContent ||
        (el as HTMLInputElement).placeholder ||
        el.textContent;
      if (!name || !name.trim()) {
        out.push(`<${el.tagName.toLowerCase()} class="${String(el.className).slice(0, 60)}">`);
      }
    }
    return out;
  });
  expect(nameless, `unnamed controls — a screen reader announces these as just "button":\n  ${nameless.join("\n  ")}`).toEqual([]);
});

test("the DOM has no duplicate ids", async ({ page }) => {
  await ready(page);
  const dupes = await page.evaluate(() => {
    const seen = new Map<string, number>();
    for (const el of Array.from(document.querySelectorAll("[id]"))) {
      seen.set(el.id, (seen.get(el.id) ?? 0) + 1);
    }
    return [...seen.entries()].filter(([, n]) => n > 1).map(([id, n]) => `#${id} ×${n}`);
  });
  // Duplicate ids break label/aria-labelledby targeting and anchor links.
  expect(dupes, `duplicate ids: ${dupes.join(", ")}`).toEqual([]);
});

test("headings form a sensible outline", async ({ page }) => {
  await ready(page);
  const { levels, problems } = await page.evaluate(() => {
    const hs = Array.from(document.querySelectorAll<HTMLElement>("h1,h2,h3,h4,h5,h6")).filter(
      (h) => h.getBoundingClientRect().height > 0,
    );
    const levels = hs.map((h) => ({ level: Number(h.tagName[1]), text: (h.textContent || "").trim().slice(0, 40) }));
    const problems: string[] = [];
    const h1s = levels.filter((l) => l.level === 1);
    if (h1s.length === 0) problems.push("no <h1> on the page");
    if (h1s.length > 1) problems.push(`${h1s.length} <h1>s: ${h1s.map((h) => h.text).join(" | ")}`);
    for (let i = 1; i < levels.length; i++) {
      if (levels[i].level - levels[i - 1].level > 1) {
        problems.push(`jumps h${levels[i - 1].level} → h${levels[i].level} at "${levels[i].text}"`);
      }
    }
    return { levels, problems };
  });
  expect(problems, `heading outline (${levels.length} headings):\n  ${problems.join("\n  ")}`).toEqual([]);
});

test("decorative icons are hidden from assistive tech, meaningful ones are labelled", async ({ page }) => {
  await ready(page);
  const bare = await page.evaluate(() => {
    const out: string[] = [];
    for (const el of Array.from(document.querySelectorAll<SVGElement>("svg"))) {
      if (el.getBoundingClientRect().width === 0) continue;
      const hidden = el.getAttribute("aria-hidden") === "true";
      const named = el.getAttribute("aria-label") || el.querySelector("title");
      // Inside a named control the icon is already covered by the parent's name.
      const inNamedControl = !!el.closest("[aria-label],[title],button,a");
      if (!hidden && !named && !inNamedControl) out.push(String(el.getAttribute("class") || "<svg>").slice(0, 50));
    }
    return out;
  });
  expect(bare, `svgs neither hidden nor named:\n  ${bare.join("\n  ")}`).toEqual([]);
});

// ------------------------------------------------------------ keyboard ---

test("focus is always visible when tabbing", async ({ page }) => {
  await ready(page);
  const invisible: string[] = [];
  for (let i = 0; i < 25; i++) {
    await page.keyboard.press("Tab");
    const info = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el || el === document.body) return null;
      const s = getComputedStyle(el);
      const ring =
        (s.outlineStyle !== "none" && parseFloat(s.outlineWidth) > 0) ||
        s.boxShadow !== "none" ||
        // Some designs mark focus with a border or background change.
        s.borderColor !== getComputedStyle(el.parentElement || document.body).borderColor;
      return {
        ring,
        tag: el.tagName.toLowerCase(),
        name: (el.getAttribute("aria-label") || el.textContent || "").trim().slice(0, 35),
      };
    });
    if (info && !info.ring) invisible.push(`${info.tag} "${info.name}"`);
  }
  expect(invisible, `focused with no visible indicator — unusable by keyboard:\n  ${invisible.join("\n  ")}`).toEqual([]);
});
