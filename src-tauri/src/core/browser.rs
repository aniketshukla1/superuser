//! The in-app browser — a real child webview, not an iframe.
//!
//! An `<iframe>` cannot show "any website": `X-Frame-Options: DENY` and
//! `frame-ancestors` mean Google, GitHub, and most banks simply refuse to render
//! inside one, and there is nothing the embedder can do about it. So the browser
//! tab is a genuine child WKWebView parented to the main window (Tauri's
//! `Window::add_child`, behind the `unstable` feature).
//!
//! The cost of that choice: a child webview is a native view layered ON TOP of the
//! HTML surface. It does not respect DOM stacking, so it cannot be occluded by a
//! modal, a dropdown, or the search palette — it has to be told to hide. The React
//! side owns that (see BrowserTab.tsx); this module just exposes the primitives.
//!
//! Position and size arrive in *logical* (CSS) pixels measured by the frontend from
//! the tab's own bounding box, so the webview tracks the panel through window
//! resizes and sidebar collapses.

use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl};

#[cfg(test)]
mod tests;

mod open;
mod dev_servers;

pub use open::*;
pub use dev_servers::*;
