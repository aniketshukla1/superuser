// An assistant's browser, live — and the user's hands on it.
//
// Frames arrive over `/ws/computer/:persona` as the browser paints them. Input goes back the
// same way and is applied only while the user holds the wheel — the server checks, because
// a socket being open is not permission, and so does this view, so nothing is sent that
// would only be refused.
//
// Closing while holding the wheel asks what to do with it. Handing it back silently could
// set the assistant loose halfway through a sign-in; keeping it silently leaves the
// assistant waiting on a window nobody can see.

export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

// CDP's modifier bits: Alt 1, Ctrl 2, Meta 4, Shift 8.
export const modifiers = (e: { altKey: boolean; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }) =>
  (e.altKey ? 1 : 0) | (e.ctrlKey ? 2 : 0) | (e.metaKey ? 4 : 0) | (e.shiftKey ? 8 : 0);

export const BUTTONS = ["left", "middle", "right"] as const;
