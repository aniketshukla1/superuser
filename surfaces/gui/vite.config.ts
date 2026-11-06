import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

// `base: "./"` makes built asset URLs relative, so the bundle loads from the `tauri://`
// origin in the desktop shell (absolute `/assets` 404s there); a server-hosted build is
// unaffected. Dev runs on a FIXED port with strictPort so the Tauri webview always
// loads the vite instance Tauri itself spawns (a drifting port would make the window load a
// stale/other server). `tauri.conf.json` devUrl must match this.
//
// The port is overridable via SUPERUSER_DEV_PORT for the case where something else on the
// machine already holds the default — strictPort means we fail rather than drift, so the
// escape hatch is an explicit choice, not a fallback. Set it for `tauri dev` too, since
// both sides have to agree.
const DEV_PORT = Number(process.env.SUPERUSER_DEV_PORT) || 1421;

// The version the splash prints, read from the one place it is set rather than typed again.
function appVersion(): string {
  try {
    const conf = fs.readFileSync(path.resolve(__dirname, "../../src-tauri/tauri.conf.json"), "utf8");
    return String(JSON.parse(conf).version || "");
  } catch {
    return "";
  }
}
export default defineConfig(({ command }) => {
  let devToken = "";
  if (command === "serve") {
    const state =
      process.env.SUPERUSER_STATE_DIR ||
      (process.platform === "win32"
        ? path.join(process.env.APPDATA || os.homedir(), "coworker")
        : path.join(os.homedir(), ".config", "coworker"));
    try {
      devToken = fs.readFileSync(path.join(state, "sidecar-8765.token"), "utf8").trim();
    } catch {
      // The Tauri dev shell injects its in-memory token at runtime. Plain browser dev
      // shows the normal startup retry until the standalone server/token file exists.
    }
  }
  return {
    base: "./",
    plugins: [react()],
    server: { port: DEV_PORT, strictPort: true },
    define: { __SUPERUSER_DEV_TOKEN__: JSON.stringify(devToken), __APP_VERSION__: JSON.stringify(appVersion()) },
    // `trace(...)` is dropped from a production build, calls and all.
    //
    // `import.meta.env.DEV` alone was not enough. It made `trace` an empty function,
    // so nothing PRINTED — but every call site survived minification with its topic,
    // its message and its data object intact, and the bundle still built those objects
    // at runtime for a function that discards them. "No output" is not the same as
    // "omitted", and shipping a bundle you can read the internal traces out of is the
    // half of it that matters here.
    //
    // Marking it pure tells the minifier the call has no side effects, so a call whose
    // result is unused — which is all of them — goes away with its arguments. Only in
    // the production build: in dev the whole point is that it runs.
    esbuild: command === "build" ? { pure: ["trace"] } : undefined,
    // Tauri CLI looks for these; harmless for the browser build.
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
  };
});
