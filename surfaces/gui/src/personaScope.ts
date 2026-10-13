// A persona is "project-scoped" only when it's code-family: an explicit directory the user
// picks, sessions grouped by project in the sidebar. Everything else (knowledge, chat) runs on
// a transparent per-conversation scratch dir, with real folders added as roots when needed —
// no folder gate, ever. (The old workspace enum — git/project/deliverable/none — collapsed
// into family; owner decision 2026-07-03, UX-DECISIONS §16.)
export function isProjectScoped(p?: { workspace?: string; family?: string }): boolean {
  return p?.family === "code";
}

// Persona naming: the product is "Superuser"; the personas are an "Assistant" family — Assistant
// (general), Code Assistant, Ops Assistant. In lists/chrome we use the SHORT label (Assistant / Code /
// Ops); the persona detail page uses the FULL family name. Backend names are left untouched (the
// API + tests keep "Superuser" / "Ops Assistant"); this is purely the display layer.

// Short label for the sidebar + top bar: "Assistant" / "Code" / "Ops" / "Chat".
export function shortPersonaName(name?: string, id?: string): string {
  if (id === "cowork") return "Assistant";
  const n = (name || id || "").trim();
  return n.replace(/\s*assistant$/i, "").trim() || n;
}

// Full name for the persona detail page. This USED to append " Assistant" to
// anything that did not already end in it, back when manifests carried terse names
// ("Ops", "Code") and the suffix was how you knew what you were looking at.
//
// Manifests now carry the whole name — "Ops Assistant", "Security Assistant" — so
// the append had nothing left to do for those, and once the roster grew into real
// job titles it started doing harm: "HR Partner Assistant", "Finance Analyst
// Assistant", "Customer Support Assistant". An HR Partner is not an assistant that
// is an HR partner, and the manifest never asked to be renamed.
//
// So the manifest name is the name. The only mapping left is the legacy `cowork` id,
// which predates the rename and still reaches this from stored state.
export function fullPersonaName(name?: string, id?: string): string {
  if (id === "cowork") return "Assistant";
  return (name || id || "").trim();
}

// Which of the topbar's tool buttons a persona should get.
//
// Each button opens a panel tab, and a button that opens an empty tab is worse than
// no button — so the rule is: show it when the persona holds the capability that
// puts something IN that tab.
//
// Both were gated on `shell`, which is right for one of them and wrong for the
// other. `browser_open` — the call the panel watches for, and the only thing that
// ever loads a page into it — is granted by the **web** capability;
// `agent.rs::browser_open_is_the_name_the_panel_watches_for` asserts exactly that.
// So Main, Customer Support and every other web-capable assistant could open a page
// in the panel and had no button to go and look at it. The agent could put something
// on screen that the user had no affordance to reach.
//
// This is the same shape as the surface-visibility guard that once compared `agent`
// against ids no persona had: a condition that names a different thing from the one
// the feature needs, with nothing in between to complain.
//
// Terminal keeps `shell`, because that one really is the shell.
export function hasCapability(p: { tools?: string[] } | undefined | null, cap: string): boolean {
  return p?.tools?.includes(cap) ?? false;
}

/** The terminal panel — shown for every persona, because it is not the agent's.
 *
 * `TerminalTab` is xterm.js over the PTY in `core/terminal.rs`, and `terminal_open`
 * takes a directory and nothing else: no persona, no capability check. It is the
 * user's own login shell. The `shell` capability decides only whether the AGENT may
 * type into it as well.
 *
 * So gating this on `shell` was answering the wrong question. It asked "may the
 * assistant run commands" and used the answer to decide "may the user open a
 * terminal" — and took the terminal away from every knowledge persona, on machines
 * where the user plainly wanted one (owner ask 2026-09-01).
 *
 * Unlike Browser, this is deliberately NOT a capability button. Browser opens a
 * panel only `browser_open` can fill, so without `web` it is empty by construction.
 * A terminal is never empty: there is always a shell to give you. */
export function showsTerminal(_p?: { tools?: string[] } | null): boolean {
  return true;
}

/** Whether the AGENT shares that shell, which is what `shell` actually grants.
 *
 * Only the wording depends on this — but the wording matters: "the shell this
 * session shares" is a false claim on a persona that cannot run a command, and it
 * is exactly the sort of thing a user would rely on. */
export function agentSharesShell(p: { tools?: string[] } | undefined | null): boolean {
  return hasCapability(p, "shell");
}

/** The panel the agent loads pages into with `browser_open`. */
export function showsBrowser(p: { tools?: string[] } | undefined | null): boolean {
  return hasCapability(p, "web");
}
