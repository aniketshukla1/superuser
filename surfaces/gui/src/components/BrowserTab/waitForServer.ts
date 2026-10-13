/** The folder the start page reads `.claude/launch.json` from, when the session
 *  itself isn't project-scoped. Remembered so it survives a restart. */
export const PROJECT_KEY = "assistant:browser-project:v1";

export function storedProject(): string {
  try {
    return localStorage.getItem(PROJECT_KEY) || "";
  } catch {
    return "";
  }
}

/** Poll a dev server until it answers, so the page loads when it's actually up
 *  rather than showing a connection error the user has to reload past. */
export async function waitForServer(url: string, timeoutMs = 30000): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      // no-cors: we only need to know something answered, not read the response.
      await fetch(url, { mode: "no-cors", cache: "no-store" });
      return true;
    } catch {
      await new Promise((r) => setTimeout(r, 400));
    }
  }
  return false;
}
