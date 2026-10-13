import { useEffect, useRef } from "react";
import { toolPress, type Props, type TabId, type ToolTab } from "./Props";

/** The topbar pressed a tool button. Three outcomes, and which one you get is exactly
 * what the button looks like it will do:
 *
 *   not open        → open it, go there
 *   open, elsewhere → go there. Pressing "Terminal" from the Files tab means "show me
 *                     the terminal", never "throw it away"
 *   open, showing   → put it away. This is the only case the button is lit for, so a
 *                     lit button closes and an unlit one opens — which is what a
 *                     toggle has to mean to be worth having
 *
 * "Put away" is the TAB, not the process. The shell keeps running and the page keeps
 * its state, so reopening puts you back where you were — the same promise the panel
 * has always made about switching away. A misfired double-press should not kill a
 * running command. */
export function useToolPress(
  openRequest: Props["openRequest"],
  tab: TabId,
  setTab: (t: TabId) => void,
  openTools: ToolTab[],
  setOpenTools: (t: ToolTab[]) => void,
) {
  // Where to go when a tool is put away: wherever you were before you reached for it.
  // Held in a ref rather than state because nothing renders from it — it is a memory, and
  // making it state would re-render the panel every time you changed tabs.
  const lastNonTool = useRef<TabId>("activity");
  useEffect(() => {
    if (tab !== "terminal" && tab !== "browser") lastNonTool.current = tab;
  }, [tab]);
  // The current tab, for the effect below to read without depending on it. Depending on
  // `tab` would re-run the request on every tab change and re-open a tool you just closed.
  const tabNow = useRef<TabId>(tab);
  useEffect(() => {
    tabNow.current = tab;
  }, [tab]);

  // Both reads come from refs so this fires on the REQUEST and nothing else. Depending on
  // `tab` or `openTools` would re-run it on every tab change and re-open what you just
  // closed; putting the setTab inside a setOpenTools updater would be a side effect in a
  // reducer, which StrictMode is entitled to run twice.
  const openNow = useRef<ToolTab[]>(openTools);
  useEffect(() => {
    openNow.current = openTools;
  }, [openTools]);

  useEffect(() => {
    if (!openRequest) return;
    const next = toolPress(openRequest.tool, tabNow.current, openNow.current, lastNonTool.current);
    setOpenTools(next.open);
    setTab(next.tab as TabId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openRequest]);
}
