import { useState } from "react";
import type { RunEvent } from "../../api";
import { Icon, ICON } from "../Icon";

/** A tool call with both halves joined. */
export interface ToolCall {
  seq: number;
  at: number;
  name: string;
  title: string;
  args: Record<string, unknown>;
  status?: string;
  result?: string;
  /** Still running: proposed with no matching finish yet. */
  pending: boolean;
  /** The run ended before this call reported back. Distinct from a failure: it
   *  did not go wrong, it was cut off — and distinct from pending, because
   *  nothing is going to arrive for it now. */
  stopped?: boolean;
}

/** The one line that says what this call was — the command, the path, the query.
 *  Mirrors the server's own summariser (commands.rs) so both surfaces read alike. */
export function titleOf(name: string, a: Record<string, unknown>): string {
  const s = (k: string) => (typeof a[k] === "string" ? (a[k] as string) : "");
  switch (name) {
    case "run_shell":
      return s("command");
    case "read_file":
    case "write_file":
    case "edit_file":
    case "list_dir":
      return s("path") || s("dir");
    case "grep":
      return s("pattern");
    case "web_search":
      return s("query");
    case "web_fetch":
      return s("url");
    default: {
      const first = Object.values(a).find((v) => typeof v === "string") as string | undefined;
      return first ? first.slice(0, 160) : "";
    }
  }
}

/** Tools that change the workspace — the Files tab badges what these touched. */
export const WRITE_TOOLS = new Set(["write_file", "edit_file", "create_file", "apply_patch"]);

export function toolCallsFrom(events: RunEvent[]): ToolCall[] {
  const calls: ToolCall[] = [];
  // Finishes arrive after their proposal and carry only the tool name, so match each
  // finish to the most recent unfinished call of that name.
  const openByName = new Map<string, ToolCall[]>();
  for (const e of events) {
    const d = (e.data ?? {}) as Record<string, unknown>;
    if (e.type === "tool_proposed") {
      const name = String(d.name ?? "");
      if (!name) continue;
      const args = (d.arguments ?? {}) as Record<string, unknown>;
      const call: ToolCall = {
        seq: e.seq,
        at: e.at,
        name,
        args,
        title: titleOf(name, args) || String(d.summary ?? ""),
        pending: true,
      };
      calls.push(call);
      const open = openByName.get(name) ?? [];
      open.push(call);
      openByName.set(name, open);
    } else if (e.type === "tool_finished") {
      const name = String(d.name ?? "");
      const call = openByName.get(name)?.shift();
      if (!call) continue;
      call.pending = false;
      call.status = typeof d.status === "string" ? d.status : undefined;
      call.result = typeof d.result_preview === "string" ? d.result_preview : undefined;
    } else if (e.type === "interrupted" || e.type === "turn_done") {
      // The run is over, so anything still open is never going to report. Leaving
      // those on their spinner is why a stopped turn went on looking like work in
      // progress: a tool interrupted mid-flight sends no `tool_finished`, and the
      // panel had no other way to learn the run had ended.
      for (const open of openByName.values()) {
        for (const call of open) {
          call.pending = false;
          call.stopped = true;
        }
      }
      openByName.clear();
    }
  }
  return calls;
}

/** Workspace-relative paths this run wrote to, newest last. */
/** The agent's "show the user this page" tool. Must match the Rust tool name —
 *  pinned on that side by `browser_open_is_the_name_the_panel_watches_for`. */
export const BROWSER_OPEN_TOOL = "browser_open";

/** The URL of the most recent `browser_open`, or "" — what the Browser tab should
 *  be showing. Reads the same call feed the Files tab reads for what changed. */
export function browserUrlWantedBy(calls: ToolCall[]): string {
  for (let i = calls.length - 1; i >= 0; i--) {
    const c = calls[i];
    if (c.name !== BROWSER_OPEN_TOOL) continue;
    // A call that errored server-side (blocked scheme, blocked host) must not
    // still yank the panel open onto a page that was never loaded.
    if (typeof c.result === "string" && c.result.startsWith("error:")) continue;
    const u = typeof c.args.url === "string" ? c.args.url : "";
    if (u) return u;
  }
  return "";
}

/**
 * Whether the panel should surface the Browser tab for a page the agent just opened.
 *
 * A rule rather than an inline condition because it is the whole of Phase 3's second
 * bullet — "an auto-opening tab that interrupts is worse than one that never opens" —
 * and because a policy nobody can test is a policy nobody can trust.
 *
 * Note what is NOT here: the page loads regardless. This decides only whether to
 * move the user, so declining costs nothing — the page is already waiting when they
 * look, and the tab wears a dot until then.
 */
export function shouldSurfaceBrowser(now: {
  /** Already on the Browser tab — nothing to surface. */
  alreadyWatching: boolean;
  /** The user has focus in a text field. Never take the keyboard mid-sentence. */
  typing: boolean;
  /** How long ago the user picked a tab by hand, in ms. */
  msSinceManualChoice: number;
}): boolean {
  if (now.alreadyWatching) return false;
  if (now.typing) return false;
  // A tab chosen seconds ago is a deliberate choice; overriding it reads as the app
  // fighting the user. Older than this and they have most likely moved on.
  if (now.msSinceManualChoice < MANUAL_TAB_GRACE_MS) return false;
  return true;
}

/** How long a hand-picked tab is protected from being replaced. */
export const MANUAL_TAB_GRACE_MS = 15_000;

export function filesTouchedBy(calls: ToolCall[]): string[] {
  const seen: string[] = [];
  for (const c of calls) {
    if (!WRITE_TOOLS.has(c.name)) continue;
    const p = typeof c.args.path === "string" ? c.args.path : "";
    if (p && !seen.includes(p)) seen.push(p);
  }
  return seen;
}

/** Running / failed / done, as a dot — the icon set has no check or cross, and at
 *  this size a coloured dot reads faster than either would anyway. */
export function statusClass(c: ToolCall): string {
  if (c.pending) return "running";
  // Cut off is not the same as failed. Colouring it red would claim something went
  // wrong when the user simply changed their mind.
  if (c.stopped) return "stopped";
  if (c.status && /err|fail/i.test(c.status)) return "bad";
  return "ok";
}

export function ToolRow({ call }: { call: ToolCall }) {
  const [open, setOpen] = useState(false);
  const hasBody = !!call.result?.trim();
  return (
    <div className={"act-row" + (call.pending ? " pending" : "")}>
      <button className="act-head" onClick={() => hasBody && setOpen((v) => !v)} disabled={!hasBody}>
        <span className={"act-status " + statusClass(call)} aria-hidden />
        <span className="act-name">{call.name}</span>
        <span className="act-title" title={call.title}>
          {call.title}
        </span>
        {call.stopped && <span className="act-stopped">stopped</span>}
        {hasBody && <Icon name={open ? "chevronDown" : "chevronRight"} size={ICON.md} className="act-chev" />}
      </button>
      {open && hasBody && <pre className="act-body">{call.result}</pre>}
    </div>
  );
}
