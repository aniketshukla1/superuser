import type { Page } from "@playwright/test";

// ─────────────────────────────────────────────────────────────────────────────
// In-page auditors. Each returns plain data so the spec can assert + report.
// ─────────────────────────────────────────────────────────────────────────────

export interface NamelessControl {
  tag: string;
  cls: string;
  html: string;
  rect: { x: number; y: number; w: number; h: number };
}

/** Interactive controls with no accessible name at all — unlabelled to a screen
 *  reader and unaddressable by `getByRole(name)`. */
export async function namelessControls(page: Page): Promise<NamelessControl[]> {
  return page.evaluate(() => {
    const sel = 'button, a[href], [role="button"], input, select, textarea, [tabindex]:not([tabindex="-1"])';
    const out: any[] = [];
    for (const el of Array.from(document.querySelectorAll(sel)) as HTMLElement[]) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue; // not visible
      if (el.getAttribute("aria-hidden") === "true") continue;
      const label =
        (el.getAttribute("aria-label") || "").trim() ||
        (el.getAttribute("title") || "").trim() ||
        (el.textContent || "").trim() ||
        (el as HTMLInputElement).placeholder ||
        (el.getAttribute("alt") || "").trim();
      const labelledby = el.getAttribute("aria-labelledby");
      const labelled =
        labelledby && labelledby.split(/\s+/).some((id) => document.getElementById(id)?.textContent?.trim());
      // A control takes its name from `label[for]`, from an ancestor <label>, or
      // (for a bare icon button) from an aria-label/title/text child.
      const forLabel =
        el.id && document.querySelector(`label[for="${CSS.escape(el.id)}"]`)?.textContent?.trim();
      const ancestorLabel = el.closest("label")?.textContent?.trim();
      if (label || labelled || forLabel || ancestorLabel) continue;
      out.push({
        tag: el.tagName.toLowerCase(),
        cls: el.className?.toString().slice(0, 60) || "",
        html: el.outerHTML.slice(0, 140).replace(/\s+/g, " "),
        rect: { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) },
      });
    }
    return out;
  });
}

/** Controls that fail WCAG 2.2 SC 2.5.8 (Target Size, Minimum).
 *
 *  The criterion is 24×24 CSS px **or** the spacing exception: an undersized target
 *  passes if a 24px-diameter circle centred on it does not intersect any other target,
 *  or the circle of any other undersized target. The first version of this helper only
 *  measured the box, so it reported a column of checkboxes with 16px of air around each
 *  as failures when the criterion actually clears them — over-reporting is as useless
 *  as under-reporting, so the exception is implemented here rather than argued about
 *  in the report. */
export async function smallTargets(page: Page, min = 24) {
  return page.evaluate((min) => {
    const sel = 'button, a[href], [role="button"], input[type="checkbox"], input[type="radio"], select';
    const boxes = (Array.from(document.querySelectorAll(sel)) as HTMLElement[])
      .map((el) => ({ el, r: el.getBoundingClientRect() }))
      .filter(({ r }) => r.width > 0 && r.height > 0);

    const radius = min / 2;
    const centre = (r: DOMRect) => ({ x: r.left + r.width / 2, y: r.top + r.height / 2 });
    // Does a circle of `radius` at `c` overlap rectangle `r`?
    const circleHitsRect = (c: { x: number; y: number }, r: DOMRect) => {
      const nx = Math.max(r.left, Math.min(c.x, r.right));
      const ny = Math.max(r.top, Math.min(c.y, r.bottom));
      return Math.hypot(c.x - nx, c.y - ny) < radius - 0.5; // 0.5px slack for rounding
    };

    const out: any[] = [];
    for (const a of boxes) {
      if (a.r.width >= min && a.r.height >= min) continue;
      const ca = centre(a.r);
      const crowded = boxes.some((b) => {
        if (b.el === a.el) return false;
        if (a.el.contains(b.el) || b.el.contains(a.el)) return false; // nested, not adjacent
        const undersized = b.r.width < min || b.r.height < min;
        return undersized
          ? Math.hypot(ca.x - centre(b.r).x, ca.y - centre(b.r).y) < min - 0.5
          : circleHitsRect(ca, b.r);
      });
      if (!crowded) continue; // passes on the spacing exception
      out.push({
        w: Math.round(a.r.width),
        h: Math.round(a.r.height),
        text: (a.el.textContent || a.el.getAttribute("aria-label") || a.el.getAttribute("title") || "")
          .trim()
          .slice(0, 40),
        cls: a.el.className?.toString().slice(0, 50) || "",
      });
    }
    return out;
  }, min);
}

/** Text that is visually clipped without an ellipsis — the reader sees a word cut mid-glyph. */
export async function clippedText(page: Page) {
  return page.evaluate(() => {
    const out: any[] = [];
    for (const el of Array.from(document.querySelectorAll("*")) as HTMLElement[]) {
      if (!el.childNodes.length) continue;
      const onlyText = Array.from(el.childNodes).every((n) => n.nodeType === 3);
      if (!onlyText) continue;
      const txt = (el.textContent || "").trim();
      if (!txt) continue;
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      const cs = getComputedStyle(el);
      // Text hidden ON PURPOSE for sighted readers: a screen-reader-only heading is a 1x1
      // clipped box by definition, and flagging it reports the accessible name as a defect.
      if (/(^|\s)sr-only(\s|$)/.test(el.className?.toString() || "")) continue;
      const overflowX = el.scrollWidth - el.clientWidth > 1;
      const overflowY = el.scrollHeight - el.clientHeight > 1;
      const ellipsis = cs.textOverflow === "ellipsis" || /line-clamp/.test(el.className?.toString() || "");
      const scrollable = /auto|scroll/.test(cs.overflowX + cs.overflowY);
      if ((overflowX || overflowY) && !ellipsis && !scrollable && cs.overflow === "hidden") {
        out.push({
          text: txt.slice(0, 60),
          cls: el.className?.toString().slice(0, 50) || "",
          box: `${Math.round(r.width)}x${Math.round(r.height)}`,
          content: `${el.scrollWidth}x${el.scrollHeight}`,
        });
      }
    }
    return out;
  });
}

/** Does the page scroll sideways? A desktop shell should never need to. */
export async function horizontalOverflow(page: Page) {
  return page.evaluate(() => {
    const d = document.documentElement;
    const offenders: any[] = [];
    if (d.scrollWidth > d.clientWidth + 1) {
      for (const el of Array.from(document.querySelectorAll("*")) as HTMLElement[]) {
        const r = el.getBoundingClientRect();
        if (r.right > d.clientWidth + 2 && r.width > 0 && getComputedStyle(el).position !== "fixed") {
          offenders.push({
            cls: el.className?.toString().slice(0, 50) || el.tagName,
            right: Math.round(r.right),
            text: (el.textContent || "").trim().slice(0, 40),
          });
        }
      }
    }
    return { doc: d.scrollWidth, view: d.clientWidth, offenders: offenders.slice(0, 8) };
  });
}
