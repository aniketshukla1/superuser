import { invoke, invokeStrict } from "./DictationStatus";
import { onTauriEvent } from "./desktopPetMascot";

// --- The interactive terminal (PTY) ---------------------------------------------
//
// The shell is shared with the agent: `run_shell` runs its commands here, so what
// streams into xterm is the agent working, not a replay of it.

export const terminalOpen = (root: string, cols: number, rows: number) =>
  invokeStrict<void>("terminal_open", { root, cols, rows });
export const terminalWrite = (data: string) => invoke<void>("terminal_write", { data });
export const terminalResize = (cols: number, rows: number) => invoke<void>("terminal_resize", { cols, rows });
export const terminalClose = () => invoke<void>("terminal_close");

/** Raw pty bytes, base64 — decode before writing to xterm (it is not UTF-8 safe to
 *  hand through the event payload as a string). */
export async function onTerminalOutput(handler: (bytes: Uint8Array) => void): Promise<() => void> {
  return onTauriEvent<string>("terminal-output", (b64) => {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    handler(out);
  });
}

export const onTerminalExit = (handler: () => void) => onTauriEvent<unknown>("terminal-exit", () => handler());
