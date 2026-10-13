// Thin bridge to the Tauri desktop shell. In the browser these are inert (isTauri() === false),
// so the SPA stays a single codebase. We use the injected `window.__TAURI__` global (the shell
// sets `withGlobalTauri`) instead of the @tauri-apps npm packages, so the browser build needs
// no Tauri dependencies.

export const isTauri = (): boolean =>
  typeof (globalThis as any).__TAURI__ !== "undefined";

// "macos" | "windows" | "linux" — injected by the shell (std::env::consts::OS) before the
// SPA loads; userAgent fallback covers browser dev. The macOS overlay-titlebar layout (and
// its traffic-light compensations) must NEVER apply on Windows, which keeps its native
// title bar (alignment bug, caught on Windows 2026-07-21).
export const platformOS = (): string => {
  const injected = (globalThis as any).__OCW_PLATFORM__;
  if (typeof injected === "string" && injected) return injected;
  return /mac/i.test(navigator.userAgent) ? "macos" : /win/i.test(navigator.userAgent) ? "windows" : "linux";
};

export type DictationStatus = {
  recording: boolean;
  model_installed: boolean;
  model_verified: boolean;
  test_passed: boolean;
  download_in_progress: boolean;
  model_name: string;
  model_bytes: number;
  supported: boolean;
  device_summary: string;
  compatibility_reason: string | null;
};

export type DictationDownloadProgress = {
  downloaded_bytes: number;
  total_bytes: number;
};

export const invoke = async <T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> => {
  const tauri = (globalThis as any).__TAURI__;
  if (!tauri?.core?.invoke) return null;
  try {
    return (await tauri.core.invoke(cmd, args)) as T;
  } catch {
    return null;
  }
};

export const invokeStrict = async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  const tauri = (globalThis as any).__TAURI__;
  if (!tauri?.core?.invoke) throw new Error("This feature is available in the desktop app.");
  return (await tauri.core.invoke(cmd, args)) as T;
};

/** Open the native macOS folder picker (Tauri only). Returns the chosen path, or null. */
export async function pickFolder(): Promise<string | null> {
  const path = await invoke<string>("pick_folder");
  return typeof path === "string" && path ? path : null;
}

/** The folder picker that works EVERYWHERE: Tauri's native dialog in the desktop shell, else the
 * sidecar-opened OS dialog (the sidecar is local, so the browser GUI still gets a real picker —
 * owner report 2026-07-04: "Browse" was desktop-only and the browser had paste-a-path only). */
export async function chooseFolder(): Promise<string | null> {
  if (isTauri()) return pickFolder();
  const { pickFolderViaServer } = await import("../api");
  return pickFolderViaServer();
}

/** Open-at-login (macOS LaunchAgent). */
export const getAutostart = () => invoke<boolean>("get_autostart");
export const setAutostart = (enabled: boolean) => invoke<boolean>("set_autostart", { enabled });

/** Keep this system awake so scheduled tasks fire while idle (caffeinate on macOS,
 * SetThreadExecutionState on Windows). Persists across restarts. */
export const getKeepAwake = () => invoke<boolean>("get_keep_awake");
export const setKeepAwake = (enabled: boolean) => invoke<boolean>("set_keep_awake", { enabled });

/** Begin native window dragging from a custom title/header region. */
export const startWindowDrag = () => invoke<boolean>("start_window_drag");
