// Which of the sidebar's nav rows are on screen, and which sit behind "More".
//
// A per-device view preference, so it lives in localStorage next to the theme and the
// collapsed-nav flag rather than in /v1/settings — it says nothing about the workspace and
// should not follow the account to another machine.
//
// Absent means "the built-in default", NOT "nothing visible": a fresh install shows the
// entries nav.ts marks `primary`. Storing the default explicitly would freeze it, so a later
// change to what ships visible would never reach anyone who had already opened the app.
import { useCallback, useEffect, useState } from "react";
import { RAIL_NAV, type AppSurface } from "./nav";

const KEY = "superuser-nav-visible";
const EVENT = "superuser:nav-visible";

/** What a fresh install shows. */
export const defaultVisible = (): AppSurface[] =>
  RAIL_NAV.filter((n) => n.tier === "primary").map((n) => n.id);

export function getNavVisible(): AppSurface[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw === null) return defaultVisible();
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return defaultVisible();
    // Filter against RAIL_NAV so an id removed from the app in a later version cannot
    // resurrect itself, and so a hand-edited value cannot inject one.
    const known = new Set(RAIL_NAV.map((n) => n.id));
    return parsed.filter((id): id is AppSurface => typeof id === "string" && known.has(id as AppSurface));
  } catch {
    return defaultVisible();
  }
}

export function setNavVisible(ids: AppSurface[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(ids));
  } catch {
    /* best effort — the sidebar still works, it just forgets next launch */
  }
  window.dispatchEvent(new CustomEvent(EVENT));
}

/** Subscribes to the event so two sidebars (or a modal and the rows behind it) agree. */
export function useNavVisible(): [AppSurface[], (ids: AppSurface[]) => void] {
  const [ids, setIds] = useState<AppSurface[]>(getNavVisible);
  useEffect(() => {
    const sync = () => setIds(getNavVisible());
    window.addEventListener(EVENT, sync);
    window.addEventListener("storage", sync);
    return () => {
      window.removeEventListener(EVENT, sync);
      window.removeEventListener("storage", sync);
    };
  }, []);
  const set = useCallback((next: AppSurface[]) => setNavVisible(next), []);
  return [ids, set];
}
