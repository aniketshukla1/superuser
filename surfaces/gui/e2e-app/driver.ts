import { WebSocketServer, type WebSocket } from "ws";
import { test as base, expect } from "@playwright/test";

export { expect };

// The runner half of the test bridge (src/testbridge.ts).
//
// The app dials US: we own the port, the app connects out. That inverts the usual
// automation setup and is the whole reason this works without shipping an inbound
// surface — there is nothing to secure in the app, and nothing to clean up if a test
// crashes. It also means the app can be started before or after the tests; whichever
// order, the bridge retries until someone answers.
//
// What this is NOT: a browser driver. There is no DOM tree, no auto-waiting, no
// selectors engine. It evaluates expressions in the real webview and calls the real
// window APIs. Use it for what only the real app can answer — the window manager, the
// OS chrome, the second window — and keep DOM-shaped assertions in the browser suites,
// which are faster and have better failure messages.

const PORT = 17999;
const CONNECT_TIMEOUT_MS = 20_000;

export interface WindowInfo {
  label: string;
  innerPhysical: { w: number; h: number };
  innerLogical: { w: number; h: number };
  outerPhysical: { w: number; h: number };
  position: { x: number; y: number };
  scaleFactor: number;
  visible: boolean;
  focused: boolean;
  fullscreen: boolean;
  minimized: boolean;
  error?: string;
}

export interface AppInfo {
  tauri: boolean;
  platform: string | null;
  windows: WindowInfo[];
  viewport: { w: number; h: number; dpr: number };
  theme: string | null;
  prefersDark: boolean;
  url: string;
}

export class App {
  private nextId = 1;
  private pending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>();

  constructor(private ws: WebSocket) {
    ws.on("message", (raw) => {
      let msg: any;
      try {
        msg = JSON.parse(String(raw));
      } catch {
        return;
      }
      if (msg.hello) return;
      const p = this.pending.get(msg.id);
      if (!p) return;
      this.pending.delete(msg.id);
      msg.ok ? p.resolve(msg.value) : p.reject(new Error(msg.error));
    });
  }

  private send<T>(cmd: Record<string, unknown>, timeoutMs = 15_000): Promise<T> {
    const id = this.nextId++;
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`the app did not answer ${JSON.stringify(cmd)} within ${timeoutMs}ms`));
      }, timeoutMs);
      this.pending.set(id, {
        resolve: (v) => {
          clearTimeout(timer);
          resolve(v);
        },
        reject: (e) => {
          clearTimeout(timer);
          reject(e);
        },
      });
      this.ws.send(JSON.stringify({ id, ...cmd }));
    });
  }

  /** Everything the bridge knows about the real windows, in one round trip. */
  info(): Promise<AppInfo> {
    return this.send<AppInfo>({ kind: "info" });
  }

  /** Evaluate an expression inside the real webview and return its (JSON) value.
   *  Written as a string rather than a function so there is no illusion that closures
   *  travel — nothing from the test's scope is available in there. */
  eval<T = unknown>(expr: string): Promise<T> {
    return this.send<T>({ kind: "eval", expr });
  }

  /** Call a method on the real window: setSize, setFullscreen, minimize, unminimize… */
  window<T = unknown>(method: string, ...args: unknown[]): Promise<T> {
    return this.send<T>({ kind: "window", method, args });
  }

  /** setSize + settle. The window manager may clamp the request (that is often the point
   *  of the test), so this returns what the window ACTUALLY became. */
  async resize(w: number, h: number): Promise<WindowInfo> {
    await this.window("setSize", w, h);
    await new Promise((r) => setTimeout(r, 450));
    const info = await this.info();
    return info.windows.find((x) => x.label === "main") ?? info.windows[0];
  }

  /** Poll an expression until it is truthy — the bridge has no auto-waiting. */
  async waitFor(expr: string, timeoutMs = 8000, everyMs = 200): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    let last: unknown;
    while (Date.now() < deadline) {
      last = await this.eval(expr).catch((e) => `error: ${e.message}`);
      if (last) return;
      await new Promise((r) => setTimeout(r, everyMs));
    }
    throw new Error(`waitFor timed out: ${expr}\nlast value: ${JSON.stringify(last)}`);
  }
}

/** One server for the whole run; each test gets the connection that is already open. */
let server: WebSocketServer | null = null;
let connected: Promise<App> | null = null;
/** What connected but wasn't the desktop window — quoted in the timeout message. */
let connectedNote = "";

function listen(): Promise<App> {
  if (connected) return connected;
  connected = new Promise<App>((resolve, reject) => {
    server = new WebSocketServer({ host: "127.0.0.1", port: PORT });
    const timer = setTimeout(() => {
      reject(
        new Error(
          `No DESKTOP window connected to the test bridge on 127.0.0.1:${PORT} within ${CONNECT_TIMEOUT_MS}ms.\n` +
            `Start the desktop app first:  npm run app\n` +
            `(The bridge only runs in dev builds; a production bundle has it compiled out.)` +
            (connectedNote ? `\n\nThese connected but were not the main desktop window:\n  ${connectedNote}` : ""),
        ),
      );
    }, CONNECT_TIMEOUT_MS);
    // Wait for the DESKTOP window specifically. Any browser tab left open on the dev
    // server runs the same bundle and dials in too — first-come-first-served would hand
    // the whole suite a Chromium tab that answers every question wrongly but plausibly.
    // The main window also has to be told apart from the mascot's second webview.
    const seen: string[] = [];
    server!.on("connection", (ws) => {
      ws.once("message", (raw) => {
        let hello: any = {};
        try {
          hello = JSON.parse(String(raw));
        } catch {
          /* not a handshake */
        }
        seen.push(`${hello.tauri ? "tauri" : "browser"} ${hello.mascot ? "(mascot) " : ""}${hello.url ?? "?"}`);
        if (!hello.tauri || hello.mascot) return; // keep listening
        clearTimeout(timer);
        resolve(new App(ws));
      });
    });
    timer.unref?.();
    server!.on("close", () => clearTimeout(timer));
    // Surface what DID connect, so "no app" and "only a browser tab" read differently.
    setTimeout(() => {
      if (seen.length) connectedNote = seen.join("\n  ");
    }, CONNECT_TIMEOUT_MS - 500);
    server.on("error", (e) => {
      clearTimeout(timer);
      reject(e);
    });
  });
  return connected;
}

export const test = base.extend<{ app: App }>({
  app: async ({}, use) => {
    const app = await listen();
    await use(app);
  },
});

test.afterAll(() => {
  server?.close();
  server = null;
  connected = null;
});
