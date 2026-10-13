import { invoke, invokeStrict } from "./DictationStatus";

// --- The in-app browser (a child webview, not an iframe) ------------------------
//
// It is a native view layered above the HTML, so it ignores DOM stacking: whenever
// something should sit on top of it (a modal, a dropdown, another tab), it has to be
// hidden explicitly. `browserVisible(false)` keeps the page and its scroll alive.

export type BrowserRect = { x: number; y: number; w: number; h: number };

export const browserOpen = (url: string, r: BrowserRect) =>
  invokeStrict<string>("browser_open", { url, x: r.x, y: r.y, w: r.w, h: r.h });
export const browserNavigate = (url: string) => invokeStrict<string>("browser_navigate", { url });
export const browserBounds = (r: BrowserRect) => invoke<void>("browser_bounds", { x: r.x, y: r.y, w: r.w, h: r.h });
export const browserVisible = (visible: boolean) => invoke<void>("browser_visible", { visible });
export const browserHistory = (action: "back" | "forward" | "reload") => invoke<void>("browser_history", { action });
export const browserClose = () => invoke<void>("browser_close");
export const browserUrl = () => invoke<string | null>("browser_url");

/** A dev server the workspace's `.claude/launch.json` knows how to start. */
export type DevServer = { name: string; command: string; port: number | null; url: string };

export const browserDevServers = (workspace: string) =>
  invoke<DevServer[]>("browser_dev_servers", { workspace });
