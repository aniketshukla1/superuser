import { test, expect, shot } from "../../e2e-ux/audit";

async function bootOverlay(page: any, hash = "#/") {
  await page.goto(`/?overlay=1${hash}`);
  await page.locator(".side, aside, nav").first().waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForTimeout(700);
  const on = await page.evaluate(() => document.querySelector(".app")?.classList.contains("tauri-overlay"));
  expect(on, "?overlay=1 did not activate the desktop layout").toBe(true);
}
test("the transcript is selectable even though the chrome is not", async ({ page }) => {
  // A desktop app whose text cannot be copied is broken; a title bar whose text selects
  // while you drag the window feels broken. Both matter, and they pull in opposite
  // directions, so assert both.
  await bootOverlay(page);
  const facts = await page.evaluate(() => {
    const pick = (sel: string) => {
      const el = document.querySelector(sel) as HTMLElement | null;
      if (!el) return null;
      const cs = getComputedStyle(el);
      return cs.userSelect || (cs as any).webkitUserSelect;
    };
    return { transcript: pick(".main-scroll"), brand: pick(".sidebar .brand"), topbar: pick(".main-topbar") };
  });
  expect(facts.transcript, `the conversation is not selectable: ${JSON.stringify(facts)}`).not.toBe("none");
  expect(facts.brand, `the title strip's text selects while dragging: ${JSON.stringify(facts)}`).toBe("none");
});

test("every backdrop blur is prefixed for WebKit", async ({ page }, testInfo) => {
  // WKWebView wanted `-webkit-backdrop-filter` for years, and the codebase prefixes it
  // in one place and not the others. Where it silently no-ops, a "glass" panel becomes
  // a translucent one and whatever is behind it shows through the text.
  await bootOverlay(page);
  const glass = await page.evaluate(() => {
    const out: any[] = [];
    for (const el of Array.from(document.querySelectorAll("*")) as HTMLElement[]) {
      const cs = getComputedStyle(el);
      const std = cs.backdropFilter;
      const pre = (cs as any).webkitBackdropFilter;
      if ((!std || std === "none") && (!pre || pre === "none")) continue;
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      out.push({ cls: el.className?.toString().slice(0, 40) || el.tagName, std, pre: pre ?? null });
    }
    return out;
  });
  await testInfo.attach("glass", { body: JSON.stringify(glass, null, 2), contentType: "application/json" });
  // In WebKit, a working blur reports a value on at least one of the two properties.
  const dead = glass.filter((g: any) => (!g.std || g.std === "none") && (!g.pre || g.pre === "none"));
  expect(dead, `backdrop blur is not applied in WebKit:\n${JSON.stringify(dead, null, 2)}`).toEqual([]);
});

// ─────────────────────────────────────────────────────────────────────────────
// The mascot window — a second, transparent, always-on-top 190×220 window the Rust
// shell opens at `index.html?mascot` (lib.rs:404) and shows when the main app
// minimizes. It is a whole surface of the product, and nothing has ever tested it.
// ─────────────────────────────────────────────────────────────────────────────

test("the mascot window renders in its own 190x220 frame", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 190, height: 220 });
  await page.goto("/?mascot");
  await page.waitForTimeout(900);
  await shot(page, "tauri-mascot");
  const facts = await page.evaluate(() => {
    const root = document.getElementById("root");
    const orb = document.querySelector(".mascot-orb") as HTMLElement | null;
    const r = orb?.getBoundingClientRect();
    return {
      petMode: document.documentElement.classList.contains("mascot-window"),
      painted: (root?.innerHTML.length ?? 0) > 40,
      // The window is `transparent(true)` with `shadow(false)`: an opaque page
      // background paints a black rectangle over the desktop instead of a floating orb.
      bodyBg: getComputedStyle(document.body).backgroundColor,
      htmlBg: getComputedStyle(document.documentElement).backgroundColor,
      orbInside: !!r && r.left >= 0 && r.top >= 0 && r.right <= 190 && r.bottom <= 220,
      docScrolls: document.documentElement.scrollWidth > 190 || document.documentElement.scrollHeight > 220,
    };
  });
  await testInfo.attach("mascot", { body: JSON.stringify(facts, null, 2), contentType: "application/json" });
  expect(facts.petMode, "?mascot did not switch main.tsx to the pet render").toBe(true);
  expect(facts.painted, "the mascot window is blank").toBe(true);
  const transparent = (c: string) => c === "transparent" || /rgba\(\s*0,\s*0,\s*0,\s*0\s*\)/.test(c);
  expect(
    { body: facts.bodyBg, html: facts.htmlBg },
    `the mascot window paints an opaque background, so a transparent always-on-top window shows as a solid rectangle over the desktop: ${JSON.stringify(facts)}`,
  ).toMatchObject({ body: expect.stringMatching(/transparent|rgba\(0, 0, 0, 0\)/) });
  void transparent;
  expect(facts.orbInside, `the mascot is clipped by its own window: ${JSON.stringify(facts)}`).toBe(true);
  expect(facts.docScrolls, `the mascot window scrolls — it has no scrollbars to scroll with`).toBe(false);
});

test("the artifact rail's :has() rule resolves in WebKit", async ({ page }) => {
  // `:has()` is the one modern selector the layout depends on for a real behaviour: it
  // widens the chat's margin when the rail opens an artifact. If the engine ignores it,
  // the artifact panel silently covers the conversation.
  await bootOverlay(page);
  const supported = await page.evaluate(() => {
    try {
      return CSS.supports("selector(:has(*))");
    } catch {
      return false;
    }
  });
  expect(supported, ":has() is unsupported here, so the artifact rail will overlap the chat").toBe(true);
});
