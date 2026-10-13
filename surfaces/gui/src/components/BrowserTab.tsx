// The Browser tab — chrome (address bar, back/forward/reload) rendered in React,
// with a real child webview from core/browser.rs painted into the empty area below.
//
// Why not an iframe: X-Frame-Options and frame-ancestors mean Google, GitHub, and
// most banks refuse to render in one. "Open any website" and iframes are mutually
// exclusive.
//
// The cost: the child webview is a NATIVE view above the HTML surface. It does not
// participate in DOM stacking, so nothing can be drawn over it — a modal or a
// dropdown would render behind it. Much of what follows exists to keep a native
// rectangle glued to a DOM rectangle, and to get it off screen the moment something
// should be on top.
//
// It opens on a start page, not a search engine: the thing you almost always want
// here is this project running, so the workspace's own dev servers are the offer.

export { BrowserTab } from "./BrowserTab/BrowserTab";
