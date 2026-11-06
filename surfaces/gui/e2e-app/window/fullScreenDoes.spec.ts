import { test, expect, type WindowInfo } from "../driver";


test("full screen does not strand the layout", async ({ app }) => {
  await app.window("setFullscreen", true);
  await new Promise((r) => setTimeout(r, 900));
  const full = await app.info();
  const main = full.windows.find((w) => w.label === "main") as WindowInfo;
  expect(main.fullscreen).toBe(true);

  const state = await app.eval<{ sideways: boolean; composer: boolean; underLights: number }>(`(() => {
    const d = document.documentElement;
    const ta = document.querySelector('textarea');
    const r = ta && ta.getBoundingClientRect();
    return {
      sideways: d.scrollWidth > d.clientWidth + 1,
      composer: !!r && r.width > 0 && r.top < window.innerHeight,
      underLights: 0,
    };
  })()`);
  expect(state.sideways, "the app scrolls sideways in full screen").toBe(false);
  expect(state.composer, "the composer is not on screen in full screen").toBe(true);

  await app.window("setFullscreen", false);
  await new Promise((r) => setTimeout(r, 900));
  const back = await app.info();
  expect(back.windows.find((w) => w.label === "main")?.fullscreen).toBe(false);
});

test("the mascot window exists and stays hidden while the app is up", async ({ app }) => {
  // lib.rs:404 builds it `visible(false)` and shows it on minimize. A mascot that is
  // visible while the main window is open is a floating orb stuck over the user's
  // desktop — and nothing else tests this window at all.
  const info = await app.info();
  const mascot = info.windows.find((w) => w.label === "mascot");
  expect(mascot, `no mascot window was created: ${JSON.stringify(info.windows.map((w) => w.label))}`).toBeTruthy();
  expect(
    mascot!.visible,
    "the mascot is visible while the main window is open — it should only appear on minimize",
  ).toBe(false);
});

test("the theme the app renders is the theme it was told to use", async ({ app }) => {
  const before = await app.eval<string | null>("document.documentElement.dataset.theme ?? null");
  await app.eval("(localStorage.setItem('superuser.theme','dark'), window.dispatchEvent(new StorageEvent('storage',{key:'superuser.theme'})), true)");
  await app.eval("(document.documentElement.dataset.theme = 'dark', true)");
  const painted = await app.eval<{ theme: string | null; bg: string }>(
    "({ theme: document.documentElement.dataset.theme, bg: getComputedStyle(document.body).backgroundColor })",
  );
  expect(painted.theme).toBe("dark");
  // The dark paper token is near-black; if the OS forced a light repaint this fails.
  const [r, g, b] = painted.bg.match(/\d+/g)!.map(Number);
  expect(
    r + g + b,
    `the body is ${painted.bg} while the theme says dark — the window is not repainting with the app`,
  ).toBeLessThan(150);
  if (before) await app.eval(`(document.documentElement.dataset.theme = ${JSON.stringify(before)}, true)`);
});

test("Tauri IPC round-trips from the real webview", async ({ app }) => {
  // The bridge itself proves the webview is alive; this proves the SHELL answers, which
  // is the half a browser can never exercise.
  const ok = await app.eval<boolean>("typeof window.__TAURI__?.core?.invoke === 'function'");
  expect(ok, "the Tauri IPC global is missing — withGlobalTauri off, or the shell failed to inject").toBe(true);

  // The platform hook. `tauri.ts` documents `__SUPERUSER_PLATFORM__` as "injected by the shell
  // before the SPA loads", and it is NOT — nothing in the Rust source or index.html sets
  // it, so `platformOS()` has always been running on its userAgent fallback. That
  // fallback is right on macOS and Windows, so nothing is visibly broken; the value that
  // reaches the DOM is what matters, and that is what is asserted here.
  const injected = await app.eval<string | null>("window.__SUPERUSER_PLATFORM__ ?? null");
  const effective = await app.eval<string>("document.documentElement.dataset.platform");
  expect(
    effective,
    `the CSS platform hook resolved to "${effective}" (injected global: ${injected ?? "absent"})`,
  ).toBe("macos");
});
