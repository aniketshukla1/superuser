// A way to drive the REAL desktop app from a test process — the thing neither
// Playwright nor the browser suites can do.
//
// Why this shape:
//
//   `tauri-driver` (the official WebDriver route) supports Linux and Windows only, so on
//   macOS there is no automation path to a WKWebView at all. The obvious alternative is a
//   test endpoint on the Rust sidecar, but that means a remote-eval surface living inside
//   a privacy-first local-first app, correctly gated forever, in a binary users install.
//   Not worth it.
//
//   So the app never listens. In DEV BUILDS ONLY this module dials *out* to a WebSocket
//   the test runner owns, on loopback. If no runner is listening — the normal case, every
//   time anyone runs `npm run app` — the socket fails to connect and this does nothing at
//   all. There is no inbound surface to secure, and `import.meta.env.DEV` means the whole
//   module is dropped from a production bundle by Vite's dead-code elimination.
//
// What it exposes is deliberately narrow: evaluate an expression in the webview, and call
// the window APIs the shell already injects. That is enough to test what only the real app
// can answer — does macOS actually enforce minWidth, does the overlay title bar clear the
// real traffic lights, does minimizing show the mascot window — without granting the test
// process anything it could not already do by typing in the dev console.

const PORT = 17999; // fixed, loopback; the runner owns it (see e2e-app/driver.ts)
const RETRY_MS = 1000;

interface Command {
  id: number;
  kind: "eval" | "window" | "info";
  /** `eval`: an expression body, evaluated and awaited. */
  expr?: string;
  /** `window`: a method on the current window, e.g. "setSize". */
  method?: string;
  args?: unknown[];
}

const tauri = () => (globalThis as any).__TAURI__;

/** The shell injects the API as a global (`withGlobalTauri`), so there is no npm
 *  dependency to add for the browser build. */
async function currentWindow(): Promise<any | null> {
  const api = tauri();
  if (!api?.window) return null;
  // Tauri v2 names it getCurrentWindow; v1 named it appWindow. Accept either.
  return api.window.getCurrentWindow?.() ?? api.window.appWindow ?? null;
}

async function describeWindows() {
  const api = tauri();
  if (!api?.window) return { tauri: false, windows: [] };
  const win = await currentWindow();
  const all = (await api.window.getAllWindows?.()) ?? (win ? [win] : []);
  const windows = [];
  for (const w of all) {
    try {
      const inner = await w.innerSize();
      const outer = await w.outerSize();
      const pos = await w.outerPosition();
      const scale = await w.scaleFactor();
      windows.push({
        label: w.label,
        // Physical pixels ÷ scale = the logical size the CSS actually sees. Reporting
        // both is the point: a Retina bug is exactly a disagreement between them.
        innerPhysical: { w: inner.width, h: inner.height },
        innerLogical: { w: Math.round(inner.width / scale), h: Math.round(inner.height / scale) },
        outerPhysical: { w: outer.width, h: outer.height },
        position: { x: pos.x, y: pos.y },
        scaleFactor: scale,
        visible: await w.isVisible(),
        focused: await w.isFocused(),
        fullscreen: await w.isFullscreen(),
        minimized: await w.isMinimized(),
      });
    } catch (e) {
      windows.push({ label: w.label, error: String(e) });
    }
  }
  return {
    tauri: true,
    platform: (globalThis as any).__OCW_PLATFORM__ ?? null,
    windows,
    viewport: { w: window.innerWidth, h: window.innerHeight, dpr: window.devicePixelRatio },
    theme: document.documentElement.dataset.theme ?? null,
    prefersDark: window.matchMedia("(prefers-color-scheme: dark)").matches,
    url: location.href,
  };
}

async function run(cmd: Command): Promise<unknown> {
  if (cmd.kind === "info") return describeWindows();

  if (cmd.kind === "window") {
    const win = await currentWindow();
    if (!win) throw new Error("no Tauri window (running in a browser?)");
    const api = tauri();
    const method = cmd.method ?? "";
    // setSize/setMinSize take a LogicalSize instance, not a plain object.
    if (method === "setSize" || method === "setMinSize") {
      const [w, h] = cmd.args as [number, number];
      const size = new api.window.LogicalSize(w, h);
      return (await win[method](size)) ?? null;
    }
    if (typeof win[method] !== "function") throw new Error(`no window method ${method}`);
    return (await win[method](...((cmd.args ?? []) as unknown[]))) ?? null;
  }

  // eval: the expression is evaluated in page scope and awaited, so a probe can read
  // computed styles and geometry the way a browser test would.
  // eslint-disable-next-line no-new-func
  const fn = new Function(`return (async () => { return (${cmd.expr}); })()`);
  return await fn();
}

function connect() {
  let ws: WebSocket;
  try {
    ws = new WebSocket(`ws://127.0.0.1:${PORT}`);
  } catch {
    setTimeout(connect, RETRY_MS);
    return;
  }
  // The handshake states what kind of page this is. Any dev server tab that happens to
  // be open runs this module too, and the runner must not mistake a browser tab for the
  // desktop window — so it is told, rather than left to guess.
  ws.onopen = () =>
    ws.send(
      JSON.stringify({
        hello: "superuser-testbridge",
        url: location.href,
        tauri: !!tauri(),
        mascot: new URLSearchParams(location.search).has("mascot"),
      }),
    );
  ws.onmessage = async (ev) => {
    let cmd: Command;
    try {
      cmd = JSON.parse(String(ev.data));
    } catch {
      return;
    }
    try {
      const value = await run(cmd);
      ws.send(JSON.stringify({ id: cmd.id, ok: true, value }));
    } catch (e) {
      ws.send(JSON.stringify({ id: cmd.id, ok: false, error: e instanceof Error ? e.message : String(e) }));
    }
  };
  // Keep trying: the runner may start after the app, and a reload drops the socket.
  ws.onclose = () => setTimeout(connect, RETRY_MS);
  ws.onerror = () => ws.close();
}

export function startTestBridge() {
  if (!import.meta.env.DEV) return; // never in a shipped bundle
  // …and never from a browser. Two reasons, both real: the browser suites would each
  // open a socket that nothing answers, and a failed WebSocket logs a console error —
  // which `tour.spec.ts` rightly treats as a failure. It also removes the possibility
  // of a stray dev-server tab answering the runner instead of the app. The desktop
  // window is the only thing this is for.
  if (!tauri()) return;
  connect();
}
