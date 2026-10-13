import { test, expect, shot, VIEWPORTS } from "../../e2e-ux/audit";

// The desktop shell's own chrome, which no browser-build test can see.
//
// On macOS the window is `titleBarStyle: "Overlay"` with `macOSPrivateApi: true`: there
// is no system title bar, the traffic lights float over the app's own top-left, and the
// app draws the title strip itself under `.app.tauri-overlay`. `?overlay=1`
// (App.tsx:1303) turns that layout on in a browser, which is the only way to reach it
// without a desktop shell — tauri-driver has no macOS support.
//
// The traffic lights are NOT part of the page. Anything the app paints underneath them
// is invisible and unclickable in the real window, and no amount of browser testing
// will show it. That is what these probes are for.

/** Where the OS draws its buttons, in CSS px, from `traffic_light_position` in lib.rs
 *  plus the standard 3×12px dots. Generous on purpose: a control that merely *touches*
 *  this box is already a problem. */
const LIGHTS = { x: 0, y: 0, w: 84, h: 40 };

async function bootOverlay(page: any, hash = "#/") {
  await page.goto(`/?overlay=1${hash}`);
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForTimeout(700);
  const on = await page.evaluate(() => document.querySelector(".app")?.classList.contains("tauri-overlay"));
  expect(on, "?overlay=1 did not activate the desktop layout").toBe(true);
}
test("the desktop layout is actually what we are testing", async ({ page }) => {
  await bootOverlay(page);
  await shot(page, "tauri-overlay-shell");
  const facts = await page.evaluate(() => {
    const brand = document.querySelector(".sidebar .brand") as HTMLElement | null;
    return {
      brandPadLeft: brand ? getComputedStyle(brand).paddingLeft : null,
      brandPadTop: brand ? getComputedStyle(brand).paddingTop : null,
    };
  });
  // If these ever stop applying, every probe below is testing the browser build and
  // quietly proving nothing.
  expect(facts.brandPadLeft, `overlay padding missing: ${JSON.stringify(facts)}`).toBe("84px");
});

for (const vp of VIEWPORTS) {
  test(`nothing is painted under the traffic lights @ ${vp.name}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await bootOverlay(page);
    const buried = await page.evaluate((L) => {
      const out: any[] = [];
      const sel = "button, a[href], input, select, textarea, [role=button], [role=tab]";
      for (const el of Array.from(document.querySelectorAll(sel)) as HTMLElement[]) {
        const r = el.getBoundingClientRect();
        if (r.width === 0 || r.height === 0) continue;
        const overlaps = r.left < L.x + L.w && r.right > L.x && r.top < L.y + L.h && r.bottom > L.y;
        if (!overlaps) continue;
        out.push({
          label: (el.getAttribute("aria-label") || el.textContent || el.tagName).trim().slice(0, 34),
          box: `${Math.round(r.x)},${Math.round(r.y)} ${Math.round(r.width)}x${Math.round(r.height)}`,
          cls: el.className?.toString().slice(0, 44) || "",
        });
      }
      return out;
    }, LIGHTS);
    await testInfo.attach("under-lights", { body: JSON.stringify(buried, null, 2), contentType: "application/json" });
    expect(
      buried,
      `these sit under the macOS window buttons — unclickable in the real window:\n${JSON.stringify(buried, null, 2)}`,
    ).toEqual([]);
  });
}

for (const vp of VIEWPORTS) {
  test(`the brand row fits its sidebar @ ${vp.name}`, async ({ page }) => {
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await bootOverlay(page);
    const fit = await page.evaluate(() => {
      const row = document.querySelector(".sidebar .brand") as HTMLElement;
      const chip = document.querySelector(".beta-tag") as HTMLElement | null;
      const bar = document.querySelector(".sidebar") as HTMLElement;
      return {
        needs: row.scrollWidth,
        has: row.clientWidth,
        chipRight: chip ? Math.round(chip.getBoundingClientRect().right) : null,
        sidebarRight: Math.round(bar.getBoundingClientRect().right),
      };
    });
    expect(
      fit.needs,
      `the title-bar row needs ${fit.needs}px of a ${fit.has}px sidebar — the overflow is sliced off its right-hand end, where the BETA chip is: ${JSON.stringify(fit)}`,
    ).toBeLessThanOrEqual(fit.has);
    if (fit.chipRight !== null) expect(fit.chipRight).toBeLessThanOrEqual(fit.sidebarRight);
  });
}

test("with the nav collapsed, the reveal cluster clears the lights", async ({ page }, testInfo) => {
  await bootOverlay(page);
  await page.keyboard.press("Meta+b"); // collapse
  await page.waitForTimeout(500);
  await shot(page, "tauri-overlay-collapsed");
  const facts = await page.evaluate((L) => {
    const topbar = document.querySelector(".main-topbar") as HTMLElement | null;
    const reveal = document.querySelector(".nav-reveal-btn") as HTMLElement | null;
    const cluster = document.querySelector("[data-testid=topbar-cluster]") as HTMLElement | null;
    const hits = (el: HTMLElement | null) => {
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return {
        box: `${Math.round(r.x)},${Math.round(r.y)} ${Math.round(r.width)}x${Math.round(r.height)}`,
        overlapsLights: r.left < L.x + L.w && r.right > L.x && r.top < L.y + L.h && r.bottom > L.y,
      };
    };
    return {
      topbarPadLeft: topbar ? getComputedStyle(topbar).paddingLeft : null,
      reveal: hits(reveal),
      cluster: hits(cluster),
    };
  }, LIGHTS);
  await testInfo.attach("collapsed", { body: JSON.stringify(facts, null, 2), contentType: "application/json" });
  expect(
    facts.cluster?.overlapsLights ?? false,
    `the collapsed [sidebar][+][search] cluster is under the window buttons: ${JSON.stringify(facts)}`,
  ).toBe(false);
});

test("full-page sub-navs start below the window buttons", async ({ page }, testInfo) => {
  // Settings and Connectors run to the window's top edge; while the nav is collapsed the
  // lights float over exactly that corner.
  await bootOverlay(page, "#/settings");
  await page.keyboard.press("Meta+b");
  await page.waitForTimeout(600);
  await shot(page, "tauri-overlay-subnav");
  const facts = await page.evaluate((L) => {
    const subnav = document.querySelector(".page-subnav") as HTMLElement | null;
    const firstItem = subnav?.querySelector("button, a[href]") as HTMLElement | null;
    const r = firstItem?.getBoundingClientRect();
    return {
      subnavPadTop: subnav ? getComputedStyle(subnav).paddingTop : null,
      firstItemTop: r ? Math.round(r.top) : null,
      buried: !!r && r.top < L.y + L.h && r.left < L.x + L.w,
    };
  }, LIGHTS);
  await testInfo.attach("subnav", { body: JSON.stringify(facts, null, 2), contentType: "application/json" });
  expect(facts.buried, `the sub-nav's first control is under the window buttons: ${JSON.stringify(facts)}`).toBe(false);
});

test("the title strip is draggable, but its controls are not", async ({ page }, testInfo) => {
  // `data-tauri-drag-region` makes a strip move the window. Two ways to get this wrong:
  // no drag region at all (a frameless window the user cannot move), or a region that
  // swallows its own buttons (every click starts a drag instead).
  await bootOverlay(page);
  const facts = await page.evaluate(() => {
    const regions = Array.from(document.querySelectorAll("[data-tauri-drag-region]")) as HTMLElement[];
    return {
      count: regions.length,
      // A button INSIDE a drag region must stop the pointer event, or it cannot be clicked.
      controlsInsideRegions: regions.flatMap((rg) =>
        (Array.from(rg.querySelectorAll("button, a[href], input")) as HTMLElement[]).map((b) => ({
          label: (b.getAttribute("aria-label") || b.textContent || b.tagName).trim().slice(0, 30),
          ownDragRegion: b.hasAttribute("data-tauri-drag-region"),
        })),
      ),
      // The strip must not be selectable — dragging a window should not select its text.
      selectable: regions.map((rg) => getComputedStyle(rg).userSelect || (getComputedStyle(rg) as any).webkitUserSelect),
    };
  });
  await testInfo.attach("drag", { body: JSON.stringify(facts, null, 2), contentType: "application/json" });
  expect(facts.count, "no draggable strip at all — a frameless window the user cannot move").toBeGreaterThan(0);
  const draggableControls = facts.controlsInsideRegions.filter((c: any) => c.ownDragRegion);
  expect(
    draggableControls,
    `these controls are themselves drag regions, so clicking them drags the window:\n${JSON.stringify(draggableControls, null, 2)}`,
  ).toEqual([]);
});
