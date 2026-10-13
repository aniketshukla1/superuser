import type { Page } from "@playwright/test";

// The app-wide /ws/events socket each page opened (UX-026 toast et al.) — specs
// push server events through it via sendAppEvent below.
export const eventSockets = new WeakMap<Page, { send: (data: string) => void }>();

/** Push an app-wide event exactly as the server would over /ws/events. Waits for
 * the GUI to have connected its socket first. */
export async function sendAppEvent(page: Page, obj: unknown): Promise<void> {
  for (let i = 0; i < 50 && !eventSockets.get(page); i++) await page.waitForTimeout(100);
  const ws = eventSockets.get(page);
  if (!ws) throw new Error("the app never opened /ws/events");
  ws.send(JSON.stringify(obj));
}
