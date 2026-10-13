import type { Page } from "@playwright/test";

/** Contrast of visible text against its painted background (WCAG 1.4.3: 4.5:1, 3:1 for large). */
export async function contrastFailures(page: Page) {
  return page.evaluate(() => {
    const lum = (c: number[]) => {
      const f = c.map((v) => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
      });
      return 0.2126 * f[0] + 0.7152 * f[1] + 0.0722 * f[2];
    };
    const parse = (s: string): number[] | null => {
      const m = s.match(/rgba?\(([^)]+)\)/);
      if (!m) return null;
      const p = m[1].split(/[,/\s]+/).filter(Boolean).map(Number);
      if (p.length >= 4 && p[3] < 0.95) return null; // translucent — skip, we can't composite reliably
      return [p[0], p[1], p[2]];
    };
    const bgOf = (el: HTMLElement): number[] => {
      let n: HTMLElement | null = el;
      while (n) {
        const c = parse(getComputedStyle(n).backgroundColor);
        if (c) return c;
        n = n.parentElement;
      }
      return [255, 255, 255];
    };
    const out: any[] = [];
    for (const el of Array.from(document.querySelectorAll("*")) as HTMLElement[]) {
      const onlyText = el.childNodes.length && Array.from(el.childNodes).every((n) => n.nodeType === 3);
      if (!onlyText) continue;
      const txt = (el.textContent || "").trim();
      if (!txt) continue;
      const r = el.getBoundingClientRect();
      if (r.width < 4 || r.height < 4) continue;
      if (r.bottom < 0 || r.top > innerHeight) continue;
      const cs = getComputedStyle(el);
      if (cs.visibility === "hidden" || Number(cs.opacity) < 0.95) continue;
      const fg = parse(cs.color);
      if (!fg) continue;
      const bg = bgOf(el);
      const l1 = lum(fg), l2 = lum(bg);
      const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
      const size = parseFloat(cs.fontSize);
      const bold = Number(cs.fontWeight) >= 700;
      const large = size >= 24 || (size >= 18.66 && bold);
      const need = large ? 3 : 4.5;
      if (ratio < need) {
        out.push({
          text: txt.slice(0, 44),
          ratio: Math.round(ratio * 100) / 100,
          need,
          size: Math.round(size * 10) / 10,
          color: cs.color,
          bg: `rgb(${bg.join(",")})`,
          cls: el.className?.toString().slice(0, 44) || "",
        });
      }
    }
    // Collapse duplicates by class+ratio so one repeated row doesn't drown the report.
    const seen = new Map<string, any>();
    for (const o of out) {
      const k = `${o.cls}|${o.ratio}`;
      if (!seen.has(k)) seen.set(k, { ...o, count: 1 });
      else seen.get(k).count++;
    }
    return [...seen.values()].sort((a, b) => a.ratio - b.ratio);
  });
}

/** Structural HTML problems a screen reader trips on. */
export async function structure(page: Page) {
  return page.evaluate(() => {
    const ids = new Map<string, number>();
    for (const el of Array.from(document.querySelectorAll("[id]"))) {
      ids.set(el.id, (ids.get(el.id) || 0) + 1);
    }
    const headings = (Array.from(document.querySelectorAll("h1,h2,h3,h4,h5,h6")) as HTMLElement[])
      .filter((h) => h.getBoundingClientRect().height > 0)
      .map((h) => ({ level: Number(h.tagName[1]), text: (h.textContent || "").trim().slice(0, 40) }));
    const imgsNoAlt = (Array.from(document.querySelectorAll("img")) as HTMLImageElement[])
      .filter((i) => !i.hasAttribute("alt"))
      .map((i) => i.src.slice(-40));
    const landmarks = {
      main: document.querySelectorAll("main, [role=main]").length,
      nav: document.querySelectorAll("nav, [role=navigation]").length,
      h1: document.querySelectorAll("h1").length,
    };
    return {
      duplicateIds: [...ids.entries()].filter(([, n]) => n > 1).map(([id, n]) => ({ id, n })),
      headings,
      imgsNoAlt,
      landmarks,
      lang: document.documentElement.lang,
      title: document.title,
    };
  });
}

/** Walk focus with Tab and record where it lands.
 *
 *  "Has a focus indicator" is measured, not guessed: the element's painted style
 *  (and its first three ancestors', for `:focus-within` rings) is sampled while
 *  focused, then again after blurring, and the two are compared. A static shadow
 *  that is always there is not an indicator; a border colour that only changes on
 *  focus is. */
export async function tabWalk(page: Page, steps = 40) {
  const seq: any[] = [];
  for (let i = 0; i < steps; i++) {
    await page.keyboard.press("Tab");
    const info = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el || el === document.body) return { tag: "body", label: "", ring: false, offscreen: false };

      // Everything a designer could plausibly use to show focus, on the control
      // and on the ancestors a `:focus-within` rule would light up.
      const sample = () => {
        const parts: string[] = [];
        let n: HTMLElement | null = el;
        for (let d = 0; d < 4 && n; d++, n = n.parentElement) {
          const cs = getComputedStyle(n);
          parts.push(
            [cs.outlineStyle, cs.outlineWidth, cs.outlineColor, cs.boxShadow, cs.borderColor, cs.backgroundColor, cs.filter].join("|"),
          );
        }
        return parts.join("//");
      };

      const focused = sample();
      el.blur();
      const blurred = sample();
      el.focus(); // put it back so Tab continues from here

      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      return {
        tag: el.tagName.toLowerCase(),
        label:
          (el.getAttribute("aria-label") || el.getAttribute("title") || el.textContent || "")
            .trim()
            .slice(0, 36),
        cls: el.className?.toString().slice(0, 44) || "",
        ring: focused !== blurred,
        outline: cs.outline,
        offscreen: r.width === 0 || r.height === 0 || r.bottom < -2 || r.top > innerHeight + 2,
      };
    });
    seq.push(info);
  }
  return seq;
}
