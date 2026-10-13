import { useCallback, useEffect, useState } from "react";
import { NAV, type AppSurface } from "./nav";

// Where you are, in the URL.
//
// Navigation used to be a `surface` union in App's state, which meant the app had
// no addressable positions at all: you couldn't link to the Brain, going Back left
// the app entirely, and a reload always dropped you on the session view. Retiring
// or renaming a surface was invisible to anything holding a reference to it,
// because nothing could hold one.
//
// This is a hash router rather than a path router because the desktop shell loads
// from `tauri://` with relative asset paths — a path router would need a server
// rewriting every deep link back to the bundle. Hash changes also give us
// back/forward for free: the browser already keeps that history.
//
// Deliberately hand-rolled. The whole requirement is "one of a handful of named
// surfaces, plus a couple of query params"; a routing library would be more
// machinery than the problem has.

/** Everything the app can be looking at. `session` is the conversation view. */
export type RouteSurface = AppSurface | "session" | "settings" | "persona";

export interface Route {
  surface: RouteSurface;
  /** A sub-tab within the surface (`?tab=timeline`). */
  tab?: string;
  /** The thing being looked at, when the surface is about one (`#/persona/ops`). */
  id?: string;
}

/** Canonical path per surface. The nav list owns the ones it knows about. */
const PATHS: Record<RouteSurface, string> = {
  session: "/",
  brain: "/brain",
  scheduled: "/automations",
  inbox: "/inbox",
  integrations: "/connectors",
  audit: "/activity",
  settings: "/settings",
  persona: "/persona",
};

/** Paths that used to mean something, or that someone might reasonably type.
 *  Kept so a link never dies just because a surface was renamed. */
const ALIASES: Record<string, RouteSurface> = {
  "/scheduled": "scheduled",
  "/automation": "scheduled",
  "/memories": "brain",
  "/memory": "brain",
  "/watches": "scheduled",
  "/connections": "integrations",
  "/audit": "audit",
  // `/chat` with no id is the session view without a specific conversation, which
  // is what `/` already means.
  "/chat": "session",
  "/session": "session",
};

const BY_PATH: Record<string, RouteSurface> = Object.fromEntries(
  (Object.entries(PATHS) as [RouteSurface, string][]).map(([s, p]) => [p, s]),
);

/** `#/brain?tab=timeline` → `{ surface: "brain", tab: "timeline" }`. */
export function parse(hash: string): Route {
  const raw = hash.replace(/^#/, "") || "/";
  const [pathPart, queryPart] = raw.split("?");
  const path = pathPart.replace(/\/+$/, "") || "/";
  const params = new URLSearchParams(queryPart || "");

  // `/persona/ops` and `/chat/s-abc` — the surfaces that address a specific thing.
  if (path.startsWith("/persona/")) {
    return { surface: "persona", id: decodeURIComponent(path.slice("/persona/".length)) };
  }
  // Which conversation you're in. `#/` stays "the session view, whichever session
  // that is" — the address you get on a fresh launch, before you've picked one.
  if (path.startsWith("/chat/")) {
    return { surface: "session", id: decodeURIComponent(path.slice("/chat/".length)) };
  }

  const surface = BY_PATH[path] ?? ALIASES[path] ?? "session";
  const tab = params.get("tab") || undefined;
  return { surface, tab };
}

export function format(route: Route): string {
  if (route.surface === "persona" && route.id) {
    return `#/persona/${encodeURIComponent(route.id)}`;
  }
  if (route.surface === "session" && route.id) {
    return `#/chat/${encodeURIComponent(route.id)}`;
  }
  const path = PATHS[route.surface] ?? "/";
  return route.tab ? `#${path}?tab=${encodeURIComponent(route.tab)}` : `#${path}`;
}

/**
 * The current route, and a way to change it.
 *
 * `replace` swaps the current entry instead of adding one — used when the app
 * corrects the URL itself (landing on a surface's default tab, say), so Back
 * still goes where the user expects rather than undoing a correction they never
 * made.
 */
export function useRoute(): [Route, (next: Route, replace?: boolean) => void] {
  const [hash, setHash] = useState(() => window.location.hash);

  useEffect(() => {
    const on = () => setHash(window.location.hash);
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);

  const navigate = useCallback((next: Route, replace = false) => {
    const target = format(next);
    if (target === (window.location.hash || "#/")) return;
    if (replace) {
      window.history.replaceState(null, "", target);
      // replaceState doesn't fire hashchange, so tell our own subscriber.
      setHash(target);
    } else {
      window.location.hash = target;
    }
  }, []);

  return [parse(hash), navigate];
}

/** The nav entries, with the path each one points at. */
export const NAV_PATHS = NAV.map((n) => ({ ...n, path: PATHS[n.id] }));
