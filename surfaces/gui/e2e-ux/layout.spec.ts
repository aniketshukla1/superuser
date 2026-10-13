import { test, expect, boot, go, shot, SURFACES, VIEWPORTS, clippedText } from "./audit";

// `tauri.conf.json` sets minWidth 880 / minHeight 600, so every size from there up
// is a layout the product ships. These specs check the ones a user can actually
// drag the window to.

/** Is this element inside the viewport, and is it the topmost thing at its own centre? */
async function reachable(page: any, selector: string) {
  return page.evaluate((sel: string) => {
    const el = document.querySelector(sel) as HTMLElement | null;
    if (!el) return { found: false };
    const r = el.getBoundingClientRect();
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height / 2;
    const top = document.elementFromPoint(cx, cy);
    return {
      found: true,
      rect: { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) },
      insideViewport: r.left >= 0 && r.top >= 0 && r.right <= innerWidth + 1 && r.bottom <= innerHeight + 1,
      // Covered by something else (a rail sitting on top, a scrim) — a click would miss.
      hitTestHitsIt: !!top && (top === el || el.contains(top) || top.contains(el)),
      coveredBy: top ? (top as HTMLElement).className?.toString().slice(0, 50) : null,
    };
  }, selector);
}

for (const vp of VIEWPORTS) {
  test(`the composer's send control is usable @ ${vp.name} (${vp.width}x${vp.height})`, async ({ page }) => {
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await boot(page);
    await page.locator("textarea").first().fill("ready to send");
    await page.waitForTimeout(300);
    await shot(page, `composer-${vp.name}`);

    // The send control is the submit button at the composer's trailing edge.
    const info = await page.evaluate(() => {
      const ta = document.querySelector("textarea");
      const bar = ta?.closest("form, .composer, div");
      const scope = bar?.parentElement || document;
      const btns = Array.from(scope.querySelectorAll("button")) as HTMLElement[];
      // The send button is the last control on the composer row; identify it by the
      // submit type or by being the right-most button on the composer's own line.
      const el =
        (scope.querySelector('button[type="submit"]') as HTMLElement) ||
        btns.sort((a, b) => b.getBoundingClientRect().right - a.getBoundingClientRect().right)[0];
      if (!el) return { found: false };
      const r = el.getBoundingClientRect();
      const cx = r.left + r.width / 2;
      const cy = r.top + r.height / 2;
      const top = document.elementFromPoint(cx, cy);
      return {
        found: true,
        label: (el.getAttribute("aria-label") || el.getAttribute("title") || el.textContent || "").trim().slice(0, 30),
        rect: { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) },
        viewport: { w: innerWidth, h: innerHeight },
        insideViewport: r.left >= 0 && r.top >= 0 && r.right <= innerWidth + 1 && r.bottom <= innerHeight + 1,
        clickable: !!top && (top === el || el.contains(top) || top.contains(el)),
        coveredBy: top ? (top as HTMLElement).className?.toString().slice(0, 60) : null,
      };
    });

    expect(
      info,
      `the send control is off-screen or covered at ${vp.width}x${vp.height}:\n${JSON.stringify(info, null, 2)}`,
    ).toMatchObject({ found: true, insideViewport: true, clickable: true });
  });
}

test("at the minimum window size the reading column is still a column", async ({ page }) => {
  await page.setViewportSize({ width: 880, height: 600 });
  await boot(page);
  const widths = await page.evaluate(() => {
    const ta = document.querySelector("textarea") as HTMLElement;
    const main = document.querySelector("main") || ta?.closest("section, div");
    return {
      viewport: innerWidth,
      composer: ta ? Math.round(ta.getBoundingClientRect().width) : -1,
      column: main ? Math.round(main.getBoundingClientRect().width) : -1,
    };
  });
  // Below ~45 characters a line stops being readable prose. At 14.5px that is ~330px.
  expect(
    widths.composer,
    `the conversation column collapses to ${widths.composer}px of an ${widths.viewport}px window — the sidebar and right rail keep their full width instead of yielding:\n${JSON.stringify(widths, null, 2)}`,
  ).toBeGreaterThan(330);
});

for (const vp of VIEWPORTS) {
  test(`nothing is clipped without an ellipsis @ ${vp.name}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await boot(page);
    const all: any[] = [];
    for (const s of SURFACES) {
      await go(page, s.hash);
      const bad = await clippedText(page);
      if (bad.length) all.push({ surface: s.name, clipped: bad });
    }
    await testInfo.attach("clipped", { body: JSON.stringify(all, null, 2), contentType: "application/json" });
    expect(all, `text cut off with no ellipsis at ${vp.width}px:\n${JSON.stringify(all, null, 2)}`).toEqual([]);
  });
}

for (const vp of VIEWPORTS) {
  test(`no control is cut off by the window edge @ ${vp.name}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await boot(page);
    const all: any[] = [];
    for (const s of SURFACES) {
      await go(page, s.hash);
      const bad = await page.evaluate(() => {
        const out: any[] = [];
        const sel = "button, a[href], input, select, textarea";
        for (const el of Array.from(document.querySelectorAll(sel)) as HTMLElement[]) {
          const r = el.getBoundingClientRect();
          if (r.width === 0 || r.height === 0) continue;
          // More than 4px of the control lives past the right edge of the window.
          if (r.right > innerWidth + 4) {
            out.push({
              tag: el.tagName.toLowerCase(),
              label: (el.getAttribute("aria-label") || el.getAttribute("placeholder") || el.textContent || "").trim().slice(0, 30),
              right: Math.round(r.right),
              viewport: innerWidth,
              cut: Math.round(r.right - innerWidth),
            });
          }
        }
        return out;
      });
      if (bad.length) all.push({ surface: s.name, controls: bad });
    }
    await testInfo.attach("cut-off", { body: JSON.stringify(all, null, 2), contentType: "application/json" });
    expect(all, `controls running off the right edge at ${vp.width}px:\n${JSON.stringify(all, null, 2)}`).toEqual([]);
  });
}

test("browser zoom to 150% does not break the shell", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await boot(page);
  // Emulating the OS/browser text-size increase a low-vision user sets.
  await page.evaluate(() => ((document.documentElement.style as any).zoom = "1.5"));
  await page.waitForTimeout(600);
  await shot(page, "zoom-150");
  const overflow = await page.evaluate(() => ({
    doc: document.documentElement.scrollWidth,
    view: document.documentElement.clientWidth,
  }));
  expect(
    overflow.doc,
    `at 150% zoom the app scrolls sideways (${JSON.stringify(overflow)})`,
  ).toBeLessThanOrEqual(overflow.view + 1);
});
