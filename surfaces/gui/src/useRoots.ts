import { addRoot, getRoots, removeRoot, type RootInfo } from "./api";
import { useCallback, useEffect, useState } from "react";

type RootsReply = { ok: boolean; error?: string; roots?: RootInfo[] };
const CHANGED = "assistant:roots-changed";

/** A session's working directories, shared by the session drawer's directories section, the
 *  settings row's folder summary and the start panel. Reads are live. Changes go to the
 *  manager, which puts them into effect on the running engine and saves them. Bumping `reloadKey`
 *  (when the drawer reopens, say) fetches them again. */
export function useRoots(sessionId: string, reloadKey?: number) {
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [roots, setRoots] = useState<RootInfo[]>([]);

  const reload = useCallback(async () => {
    try {
      setRoots(await getRoots(sessionId));
    } catch {
      setRoots([]);
    }
  }, [sessionId]);

  useEffect(() => {
    void reload();
  }, [reload, reloadKey]);

  // Every component that shows roots (the composer chip, the start panel) holds its own copy
  // of this hook, so a change made through one is announced and the others fetch again.
  useEffect(() => {
    const onChanged = (event: Event) => {
      if ((event as CustomEvent).detail === sessionId) void reload();
    };
    window.addEventListener(CHANGED, onChanged);
    return () => window.removeEventListener(CHANGED, onChanged);
  }, [sessionId, reload]);

  // Takes the manager's reply; true when the change went through.
  const settle = (reply: RootsReply): boolean => {
    if (!reply.ok || !reply.roots) {
      setError(reply.error || "could not update directories");
      void reload();
      return false;
    }
    setRoots(reply.roots);
    setError("");
    window.dispatchEvent(new CustomEvent(CHANGED, { detail: sessionId }));
    return true;
  };

  // One change, with `busy` raised while it runs.
  const run = async (change: () => Promise<RootsReply>): Promise<boolean> => {
    setBusy(true);
    const took = settle(await change());
    setBusy(false);
    return took;
  };

  const add = useCallback(
    (path: string, writable: boolean) => run(() => addRoot(sessionId, path, writable)),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [sessionId],
  );

  // Adding a root again with the other access level changes it in place.
  const toggleAccess = useCallback(
    async (root: RootInfo) => {
      if (!root.primary) await run(() => addRoot(sessionId, root.path, !root.writable));
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [sessionId],
  );

  const remove = useCallback(
    async (path: string) => {
      await run(() => removeRoot(sessionId, path));
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [sessionId],
  );

  return { addRoot: add, removeRoot: remove, toggleAccess, reload, roots, busy, error };
}
