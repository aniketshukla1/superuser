// Making a parked Inbox item readable before anyone is asked to decide on it.
//
// Since 2026-07-14 an approval carries its tool and arguments in `data`, and the card dresses
// it the way the live transcript card does. Everything parked before that, and every MCP tool
// call, arrives as a title and a body the server wrote for itself:
//
//   title  "Run codex on codex"
//   body   {"prompt":"\\u2019I\\u2019m Aniket…\\u201C\\u2019m looking for…"}
//
// and the card printed both as they were — a heading that named the tool twice, and thirteen
// lines of JSON whose quotes had been escaped one time too many. Five of those in a row, each
// with the same purple Approve, is how something gets approved without being read.
//
// This recovers what the item is asking for from the shapes those rows actually take.

import type { InboxItem } from "./api";
import { toMillis } from "./when";

/** Undo a second round of string escaping: `\\u2019` → ’, `\\n` → a newline. */
export function unescapeTwice(s: string): string {
  return s
    .replace(/\\u([0-9a-fA-F]{4})/g, (_, hex: string) => String.fromCharCode(parseInt(hex, 16)))
    .replace(/\\n/g, "\n")
    .replace(/\\t/g, "\t")
    .replace(/\\"/g, '"');
}

function parseArgs(body: string): Record<string, unknown> | null {
  const t = (body || "").trim();
  if (!t.startsWith("{")) return null;
  try {
    const v = JSON.parse(t);
    if (!v || typeof v !== "object" || Array.isArray(v)) return null;
    const out: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(v)) out[k] = typeof val === "string" ? unescapeTwice(val) : val;
    return out;
  } catch {
    return null;
  }
}

export interface ParkedApproval {
  /** The tool the call is for. */
  tool: string;
  /** The MCP server it goes through, when it is not a built-in. */
  server?: string;
  args: Record<string, unknown>;
}

/** What an approval is for, from `data` when it has it and from the title and body when not. */
export function parkedApproval(item: InboxItem): ParkedApproval | null {
  if (item.kind !== "approval") return null;
  if (item.data?.tool) return { tool: String(item.data.tool), args: (item.data.arguments ?? {}) as Record<string, unknown> };
  const args = parseArgs(item.body) ?? {};
  const shell = item.title.match(/^Run:\s+([\s\S]+)$/);
  if (shell) return { tool: "run_shell", args: { command: typeof args.command === "string" ? args.command : shell[1], ...args } };
  const mcp = item.title.match(/^Run (\S+) on (\S+)$/);
  if (mcp) return { tool: mcp[1], server: mcp[2], args };
  // "Approve: run_shell" over a plain-text body — the oldest shape: the tool in the title and
  // its one argument, unwrapped, as the body.
  const named = item.title.match(/^Approve:\s+([\w.-]+)$/);
  if (named) {
    const plain = (item.body || "").trim();
    return {
      tool: named[1],
      args: Object.keys(args).length ? args : plain ? { [named[1] === "run_shell" ? "command" : "text"]: plain } : {},
    };
  }
  return Object.keys(args).length ? { tool: "", args } : null;
}

/** The argument that IS the request — what a person needs to read — and how to show it. */
export function primaryArg(args: Record<string, unknown>): { key: string; text: string; mono: boolean } | null {
  for (const [key, mono] of [
    ["command", true],
    ["prompt", false],
    ["content", true],
    ["text", false],
    ["message", false],
    ["query", false],
    ["url", true],
    ["path", true],
  ] as const) {
    const v = args[key];
    if (typeof v === "string" && v.trim()) return { key, text: v, mono };
  }
  return null;
}

/** Everything else, as short "key: value" lines — settings like `approval-policy`, which matter
 *  but are not the request. Nested values are flattened one level. */
export function secondaryArgs(args: Record<string, unknown>, primaryKey?: string): string[] {
  const lines: string[] = [];
  for (const [k, v] of Object.entries(args)) {
    if (k === primaryKey) continue;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      for (const [k2, v2] of Object.entries(v as Record<string, unknown>)) {
        lines.push(`${k2}: ${typeof v2 === "string" ? v2 : JSON.stringify(v2)}`);
      }
    } else {
      lines.push(`${k}: ${typeof v === "string" ? v : JSON.stringify(v)}`);
    }
  }
  return lines.map((l) => (l.length > 120 ? `${l.slice(0, 119)}…` : l));
}

/** Who is asking, in words. Automation runs are stored as `__run__run-ed914dd092`, and eval
 *  sessions by their harness id — neither is a name. */
export function requesterLabel(item: InboxItem): string {
  const taskTitle = typeof item.data?.task_title === "string" ? item.data.task_title : "";
  const title = item.session_title || "";
  if (taskTitle) return `${taskTitle} (automation)`;
  if (/^__run__/.test(title) || /^__run__/.test(item.session_id)) return "An automation run";
  return title || "A conversation";
}

/** Older than a day and still waiting: the agent that asked has long since moved on or given up. */
export const STALE_APPROVAL_MS = 24 * 60 * 60 * 1000;

export function isStaleApproval(item: InboxItem, now = Date.now()): boolean {
  if (item.kind !== "approval") return false;
  // `created_at` is typed as a string and sent by the core as epoch seconds (inbox.rs stores an
  // f64). `Date.parse` of a number is NaN, so a raw parse here never found anything stale.
  const t = toMillis(item.created_at as unknown as number | string);
  return t !== null && now - t > STALE_APPROVAL_MS;
}
