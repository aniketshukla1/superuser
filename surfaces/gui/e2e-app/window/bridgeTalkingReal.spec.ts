import { test, expect } from "../driver";


// Questions only the running desktop app can answer. Everything here would pass
// vacuously in a browser, which is why none of it was covered before.

test("the bridge is talking to a real Tauri window", async ({ app }) => {
  const info = await app.info();
  expect(info.tauri, "not a Tauri webview — is this the browser dev server?").toBe(true);
  const main = info.windows.find((w) => w.label === "main");
  expect(main, `no window labelled "main": ${JSON.stringify(info.windows)}`).toBeTruthy();
});

test("the app holds together at, and below, its declared minimum size", async ({ app }) => {
  // What this can and cannot prove, stated up front: `minWidth: 880, minHeight: 600` is
  // an NSWindow constraint on USER resizing. A programmatic `set_size` is not clamped by
  // it — measured, not assumed: asking for 400x300 gets exactly 400x300. So this cannot
  // verify the drag floor, and pretending otherwise would be a test that asserts a
  // coincidence. What it can verify is that the layout survives the sizes it can reach.
  const min = await app.resize(880, 600);
  expect(min.innerLogical).toMatchObject({ w: 880, h: 600 });
  const atMin = await app.eval<{ sideways: boolean; chatW: number; sendClickable: boolean }>(`(() => {
    const d = document.documentElement;
    const send = document.querySelector('button[aria-label="Send"]');
    const r = send && send.getBoundingClientRect();
    const top = r && document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    const chat = document.querySelector('.main-chat');
    return {
      sideways: d.scrollWidth > d.clientWidth + 1,
      chatW: chat ? Math.round(chat.getBoundingClientRect().width) : -1,
      sendClickable: !!top && !!send && (top === send || send.contains(top) || top.contains(send)),
    };
  })()`);
  expect(atMin.sideways, "the app scrolls sideways at its own minimum size").toBe(false);
  expect(atMin.sendClickable, "Send is unreachable at the minimum size").toBe(true);
  expect(atMin.chatW, `the reading column is ${atMin.chatW}px at the minimum size`).toBeGreaterThan(360);

  // Below the minimum is reachable programmatically, and a display change or a stray
  // setSize can put a user there. It must degrade, not break.
  const tiny = await app.resize(400, 300);
  expect(tiny.innerLogical.w).toBe(400); // documents the bypass rather than asserting a clamp
  const atTiny = await app.eval<{ sideways: boolean; composer: boolean }>(`(() => {
    const d = document.documentElement;
    const ta = document.querySelector('textarea');
    return { sideways: d.scrollWidth > d.clientWidth + 1, composer: !!ta && ta.getBoundingClientRect().width > 0 };
  })()`);
  expect(atTiny.sideways, "the app scrolls sideways below its minimum size").toBe(false);
  expect(atTiny.composer, "the composer disappears below the minimum size").toBe(true);

  await app.resize(1100, 760); // leave it as we found it
});

test("the CSS viewport matches the window the OS actually gave us", async ({ app }) => {
  // The bug this catches is a Retina one: physical pixels ÷ scaleFactor must equal what
  // CSS sees. A mismatch means every layout breakpoint is measured against the wrong
  // number on a 2x display, which is most Macs.
  const target = await app.resize(1000, 700);
  const viewport = await app.eval<{ w: number; h: number; dpr: number }>(
    "({ w: window.innerWidth, h: window.innerHeight, dpr: window.devicePixelRatio })",
  );
  expect(viewport.dpr).toBe(target.scaleFactor);
  // ±2px: the webview's inner size excludes nothing on an overlay title bar, but
  // rounding at fractional scale factors is real.
  expect(Math.abs(viewport.w - target.innerLogical.w)).toBeLessThanOrEqual(2);
  await app.resize(1100, 760);
});

test("the app's own title bar clears the real traffic lights", async ({ app }) => {
  // e2e-tauri/ asserts this against a SIMULATED light position in a browser. This is the
  // same property measured in the window that actually has them.
  await app.resize(1100, 760);
  const overlay = await app.eval<boolean>(
    "document.querySelector('.app')?.classList.contains('tauri-overlay') ?? false",
  );
  expect(overlay, "the real app is not in the overlay layout — is titleBarStyle still Overlay?").toBe(true);

  const buried = await app.eval<unknown[]>(`(() => {
    const L = { x: 0, y: 0, w: 84, h: 40 };
    const out = [];
    for (const el of document.querySelectorAll('button, a[href], input, select, textarea, [role=button]')) {
      const r = el.getBoundingClientRect();
      if (!r.width || !r.height) continue;
      if (r.left < L.x + L.w && r.right > L.x && r.top < L.y + L.h && r.bottom > L.y)
        out.push(((el.getAttribute('aria-label') || el.textContent || el.tagName) + '').trim().slice(0, 30));
    }
    return out;
  })()`);
  expect(
    buried,
    `these controls sit under the macOS window buttons in the real window: ${JSON.stringify(buried)}`,
  ).toEqual([]);
});

test("the brand row still fits once the OS has had its say", async ({ app }) => {
  await app.resize(880, 600); // the tightest supported window
  // …and at the narrowest the sidebar itself can be dragged to. The window being
  // small is not the binding case: the sidebar has its own minimum, and at the old
  // 200px the Workspace icon was clipped off the edge while the window was still
  // full size. Measure where the floor actually is.
  await app.eval(`(() => {
    const app = document.querySelector('.app');
    const min = 224; // App.tsx usePanelWidth min — kept in step by this test failing
    app.style.setProperty('--nav-w', min + 'px');
  })()`);
  // Measured control by control, not with scrollWidth. An earlier version of this
  // test compared scrollWidth to clientWidth and could not fail: the row is
  // inline-flex, which absorbs the overflow into its own scroll box, so a button
  // sitting past the sidebar's edge reported a tidy fit. What shipped behind that
  // green test was a search icon nobody could see.
  const fit = await app.eval<{
    needed: number;
    available: number;
    slack: number;
    overflowing: string[];
  }>(`(() => {
    const row = document.querySelector('.sidebar .brand');
    const bar = document.querySelector('.sidebar');
    const cs = getComputedStyle(row);
    const barRight = bar.getBoundingClientRect().right;
    const controls = [...row.querySelectorAll('button')];
    // What the row NEEDS, summed. Distance to the right edge measures nothing here:
    // the \`flex-1\` spacer closes it to ~0 whether the row fits or overflows.
    const gap = parseFloat(cs.gap) || 0;
    const items = [...row.children].filter(
      (c) => c.getBoundingClientRect().width > 0 && !c.classList.contains('flex-1'),
    );
    const needed = items.reduce((n, c) => n + c.getBoundingClientRect().width, 0) + gap * (items.length - 1);
    const available =
      row.getBoundingClientRect().width - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
    return {
      needed: Math.round(needed),
      available: Math.round(available),
      slack: Math.round(available - needed),
      overflowing: controls
        .filter((b) => b.getBoundingClientRect().right > barRight)
        .map((b) => b.getAttribute('aria-label') || '(unlabelled)'),
    };
  })()`);

  expect(
    fit.overflowing,
    `these title-bar controls hang past the sidebar's edge: ${fit.overflowing.join(", ")}`,
  ).toEqual([]);
  expect(
    fit.needed,
    `the title-bar row needs ${fit.needed}px of ${fit.available}px — after the traffic lights there is not room for every control`,
  ).toBeLessThanOrEqual(fit.available);
  // Not merely fitting: fitting with somewhere to go. At 1px this row has been one
  // font-metric change away from breaking, twice.
  expect(fit.slack, `only ${fit.slack}px of slack in the title-bar row at the sidebar's minimum width`).toBeGreaterThanOrEqual(6);
  await app.eval(`document.querySelector('.app').style.removeProperty('--nav-w')`);
  await app.resize(1100, 760);
});

test("the composer's send control is reachable at the minimum size", async ({ app }) => {
  // The §4 fix, verified where it matters: WebKit's flex-wrap and the real rail, not
  // Chromium's.
  await app.resize(880, 600);
  const send = await app.eval<{ found: boolean; clickable: boolean; coveredBy: string | null }>(`(() => {
    const b = document.querySelector('button[aria-label="Send"]');
    if (!b) return { found: false, clickable: false, coveredBy: null };
    const r = b.getBoundingClientRect();
    const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    return {
      found: true,
      clickable: !!top && (top === b || b.contains(top) || top.contains(b)),
      coveredBy: top ? (top.className + '').slice(0, 40) : null,
    };
  })()`);
  expect(send.found, "no Send button in the real window").toBe(true);
  expect(send.clickable, `Send is covered by "${send.coveredBy}" in the real window at 880x600`).toBe(true);
  await app.resize(1100, 760);
});
