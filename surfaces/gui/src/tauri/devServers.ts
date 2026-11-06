import { invoke, invokeStrict } from "./DictationStatus";
import { onTauriEvent } from "./desktopPetMascot";

// Dev servers: the hidden terminal behind the browser. Each has its own pty in the folder the
// start page read, NOT the shared shell, which sits in the session's workspace; there a
// relative command such as `npm --prefix frontend …` ran against the wrong project.

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

/** Open a URL in the user's browser as well as possible: through the shell's opener plugin
 *  when it is there, otherwise `window.open`. Callers should show the URL too, so it can be
 *  copied if neither works (the desktop webview has no opener plugin wired up yet). */
export function openExternal(url: string): void {
  const inNewTab = () => window.open(url, "_blank", "noopener,noreferrer");
  const opener = (globalThis as any).__TAURI__?.opener;
  if (opener?.openUrl) opener.openUrl(url).catch(inNewTab);
  else inNewTab();
}
