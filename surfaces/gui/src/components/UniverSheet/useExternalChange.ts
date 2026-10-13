import { useEffect, useState, type RefObject } from "react";
import { externalChangeAction } from "./externalChangeAction";

/** The workbook the editor is currently built from. It lags `dataUrl` on purpose:
 * when the agent edits the file we want the panel to show it, but rebuilding the
 * editor under someone who is typing in it is the Phase 3 lesson again — an
 * interruption is worse than a delay. So an external change is adopted silently
 * when the editor is idle, and offered as a one-click reload when it is not. */
export function useExternalChange(dataUrl: string, host: RefObject<HTMLDivElement>) {
  const [loadedUrl, setLoadedUrl] = useState(dataUrl);
  const [stale, setStale] = useState(false);

  useEffect(() => {
    if (dataUrl === loadedUrl) return;
    // Our own saves never reach here — they don't re-read the file (see `onSaved`),
    // so a changed `dataUrl` means someone else wrote to it: the agent, or a tool.
    const action = externalChangeAction({
      changed: true,
      busy: !!host.current && host.current.contains(document.activeElement),
    });
    if (action === "offer") {
      setStale(true);
      return;
    }
    setStale(false);
    setLoadedUrl(dataUrl);
  }, [dataUrl, loadedUrl]);

  const reload = () => {
    setStale(false);
    setLoadedUrl(dataUrl);
  };
  return { loadedUrl, stale, reload };
}
