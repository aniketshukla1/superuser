// The two things an assistant's browser can block a turn on — the user's hands, or one
// value — as transcript items, and how they settle. Kept out of App.tsx so they can be
// tested without standing the whole app up.

import type { Item } from "./types";

type Help = Extract<Item, { kind: "computer_help" }>;
type Secret = Extract<Item, { kind: "computer_secret" }>;

// `findLastIndex` is ES2023, past what the app targets.
function newestOpen(items: Item[], kind: "computer_help" | "computer_secret"): number {
  let at = -1;
  items.forEach((it, i) => {
    if (it.kind === kind && !it.resolved) at = i;
  });
  return at;
}

/** Settle the newest open request for the user's hands. */
export function resolveHelp(items: Item[], how: NonNullable<Help["resolved"]>): Item[] {
  const i = newestOpen(items, "computer_help");
  if (i < 0) return items;
  const copy = [...items];
  copy[i] = { ...(copy[i] as Help), resolved: how };
  return copy;
}

/** Settle the newest open request for a value. */
export function resolveSecret(items: Item[], how: NonNullable<Secret["resolved"]>): Item[] {
  const i = newestOpen(items, "computer_secret");
  if (i < 0) return items;
  const copy = [...items];
  copy[i] = { ...(copy[i] as Secret), resolved: how };
  return copy;
}

/** A finished turn is waiting on nobody: whatever it asked the browser's user for has lapsed. */
export function endBrowserAsks(items: Item[]): Item[] {
  if (!items.some((it) => (it.kind === "computer_help" || it.kind === "computer_secret") && !it.resolved)) return items;
  return items.map((it) =>
    (it.kind === "computer_help" || it.kind === "computer_secret") && !it.resolved ? { ...it, resolved: "ended" as const } : it,
  );
}
