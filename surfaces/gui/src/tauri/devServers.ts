import { invoke, invokeStrict } from "./DictationStatus";
import { onTauriEvent } from "./desktopPetMascot";

// --- Dev servers: the hidden terminal behind the browser -------------------------
//
// Their own pty rooted at the folder the start page read, NOT the shared shell —
// that one is rooted at the session's workspace, so a relative command like
// `npm --prefix frontend …` ran against the wrong project entirely.

export const devserverStart = (name: string, cwd: string, command: string) =>
  invokeStrict<void>("devserver_start", { name, cwd, command });
export const devserverStop = (name: string) => invoke<void>("devserver_stop", { name });
export const devserverRunning = () => invoke<string[]>("devserver_running");

export const onDevserverOutput = (handler: (e: { name: string; data: string }) => void) =>
  onTauriEvent<{ name: string; data: string }>("devserver-output", handler);
export const onDevserverExit = (handler: (e: { name: string; code: number }) => void) =>
  onTauriEvent<{ name: string; code: number }>("devserver-exit", handler);

/** Fires on every navigation the page performs — redirects and in-page links
 *  included, which is why the address bar follows the page rather than the typing. */
export const onBrowserNav = (handler: (url: string) => void) => onTauriEvent<string>("browser-nav", handler);

/** Best-effort open a URL in the user's browser. Uses the Tauri opener plugin if present, else
 * `window.open`. The caller should also render the raw URL so it stays copyable if both no-op
 * (the desktop webview has no opener plugin wired yet). */
export function openExternal(url: string): void {
  const opener = (globalThis as any).__TAURI__?.opener;
  if (opener?.openUrl) {
    opener.openUrl(url).catch(() => window.open(url, "_blank", "noopener,noreferrer"));
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}
