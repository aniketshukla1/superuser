// Drag-to-resize for the two side panels.
//
// A 5px strip sitting on a panel's edge. Pointer capture keeps the drag alive when
// the cursor outruns the strip, and the width is persisted per panel so the layout
// survives a restart.
//
// While a drag is live it broadcasts `su-panel-drag`. The browser tab listens: its
// child webview is a native view OUTSIDE the DOM, so once the cursor crosses over it
// the OS routes the mouse there and pointer capture can't follow. Hiding it for the
// duration is what keeps the drag from dying halfway across the screen.

export { PANEL_DRAG_EVENT, CHAT_MIN_W, CHAT_COMFORT_W, useWindowWidth, fitBeside, usePanelWidth } from "./ResizeHandle/usePanelWidth";
export { ResizeHandle } from "./ResizeHandle/ResizeHandle";
