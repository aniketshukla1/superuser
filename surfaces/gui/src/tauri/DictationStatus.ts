// The basics of the bridge to the Tauri desktop shell: telling whether it is there, which
// platform it runs on, and calling its commands. In a browser every call is inert, so one
// codebase serves both; see tauri.ts.

const tauriGlobal = () => (globalThis as any).__TAURI__;

export const isTauri = (): boolean => tauriGlobal() !== undefined;

/** "macos", "windows" or "linux". The shell may inject it before the page loads (from
 *  std::env::consts::OS); otherwise the user agent decides, which covers the browser in
 *  development. The macOS overlay title bar, and the room it leaves for the window buttons,
 *  must NEVER apply on Windows, which keeps its native title bar (caught there 2026-07-21). */
export function platformOS(): string {
  const injected = (globalThis as any).__SUPERUSER_PLATFORM__;
  if (typeof injected === "string" && injected) return injected;
  const agent = navigator.userAgent;
  if (/mac/i.test(agent)) return "macos";
  return /win/i.test(agent) ? "windows" : "linux";
}

/** What the voice dictation engine reports. */
export type DictationStatus = {
  supported: boolean;
  compatibility_reason: string | null;
  device_summary: string;
  model_name: string;
  model_bytes: number;
  model_installed: boolean;
  model_verified: boolean;
  download_in_progress: boolean;
  test_passed: boolean;
  recording: boolean;
};

export type DictationDownloadProgress = {
  total_bytes: number;
  downloaded_bytes: number;
};

/** Run a shell command; null outside the desktop app or when the command fails. */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  const run = tauriGlobal()?.core?.invoke;
  if (!run) return null;
  try {
    return (await run(cmd, args)) as T;
  } catch {
    return null;
  }
}

/** Run a shell command and let its failure through; outside the desktop app it fails at once. */
export async function invokeStrict<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const run = tauriGlobal()?.core?.invoke;
  if (!run) throw new Error("This feature is available in the desktop app.");
  return (await run(cmd, args)) as T;
}

/** The system folder picker, in the desktop app only; the chosen path or null. */
export async function pickFolder(): Promise<string | null> {
  const chosen = await invoke<string>("pick_folder");
  return typeof chosen === "string" && chosen !== "" ? chosen : null;
}

/** A folder picker that works EVERYWHERE: the desktop's own dialog, or else the one the local
 *  sidecar opens, so the browser still gets a real picker (owner, 2026-07-04: "Browse" was
 *  desktop-only and the browser could only take a pasted path). */
export async function chooseFolder(): Promise<string | null> {
  if (isTauri()) return pickFolder();
  const { pickFolderViaServer } = await import("../api");
  return pickFolderViaServer();
}

/** Open at login (a LaunchAgent on macOS). */
export const getAutostart = () => invoke<boolean>("get_autostart");
export const setAutostart = (enabled: boolean) => invoke<boolean>("set_autostart", { enabled });

/** Keep the computer awake so scheduled tasks fire while it is idle (caffeinate on macOS,
 *  SetThreadExecutionState on Windows). Remembered across restarts. */
export const getKeepAwake = () => invoke<boolean>("get_keep_awake");
export const setKeepAwake = (enabled: boolean) => invoke<boolean>("set_keep_awake", { enabled });

/** Start dragging the window from a custom title or header area. */
export const startWindowDrag = () => invoke<boolean>("start_window_drag");
