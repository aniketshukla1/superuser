import type { MemoryRow } from "../../api";
import { clock, shortDate } from "../../when";

// The Memory Tree, made visible.
//
// The engine has always been there — it's what makes an assistant know your context
// — but there was no way to see what it had learned. An invisible memory is one you
// can neither trust nor correct, so this shows every saved memory, lets you search
// them, and lets you delete the wrong ones.
//
// The timeline and the topic wiki are siblings, not children: `BrainView` mounts all
// three side by side. This one owns the vault (`core/memory.rs`) and nothing else.
//
// Two panes: the list stays lightweight (the API omits bodies), and picking a row
// fetches that one memory in full.

// Group by the source that produced them (GitHub, Notion, a session…) — 125 flat
// rows tell you nothing about where your context came from.
export function groupByCategory(rows: MemoryRow[]): [string, MemoryRow[]][] {
  const by = new Map<string, MemoryRow[]>();
  for (const m of rows) {
    const k = m.category?.trim() || "Uncategorised";
    by.set(k, [...(by.get(k) ?? []), m]);
  }
  return [...by.entries()].sort((a, b) => b[1].length - a[1].length);
}

/** "Repo: aniketshukla1/mem0" → a kind and a name. A sync writes its kind into every title,
 *  so a group of ninety rows all began with the same five characters, and at a narrow width
 *  that prefix was all that fit — every row read "Repo: aniketshukla1/…". The kind is still
 *  shown; it just stops being what the eye reads first. */
export function titleParts(title: string): { kind?: string; name: string } {
  const m = title.match(/^([A-Z][\w-]*(?: [\w-]+)?):\s+(.+)$/);
  return m ? { kind: m[1], name: m[2] } : { name: title };
}

/** ISO stamps inside a memory's text, said the way the rest of the app says a time. The file
 *  keeps the stamp; only the page reads it for a person. */
export function readableStamps(text: string): string {
  return text.replace(/\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})\b/g, (iso) =>
    Number.isNaN(Date.parse(iso)) ? iso : `${shortDate(iso)}, ${clock(iso)}`,
  );
}
