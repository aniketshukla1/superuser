import { useState } from "react";
import type { Item } from "../../types";
import type { HumanLine } from "../../humanize";
import { Icon, ICON } from "../Icon";

export function shortArgs(args: any): string {
  if (!args || typeof args !== "object") return "";
  return Object.entries(args)
    .map(([k, v]) => {
      let s = typeof v === "string" ? v : JSON.stringify(v);
      if (s.length > 96) s = s.slice(0, 95) + "...";
      return `${k}=${s.replace(/\n/g, " ")}`;
    })
    .join("  ");
}

// Human verbs kept for the §25 grant lines (the card title now comes from humanize.ts).
export const TOOL_VERBS: Record<string, string> = {
  write_file: "Write a file",
  replace_in_file: "Edit a file",
  apply_patch: "Apply a patch",
  apply_unified_diff: "Apply a patch",
  run_shell: "Run a command",
  send_message: "Send a message",
  send_file: "Send a file",
};

// §35: routine workspace writes render as a compact ROW; everything else is a full card.
export const FILE_WRITES = new Set(["write_file", "replace_in_file", "apply_patch", "apply_unified_diff"]);
// Actions that leave the Mac get the warm border + explicit destination note.
export const EXTERNAL = new Set(["send_message", "send_file"]);

export type ApprovalItem = Extract<Item, { kind: "approval" }>;

// Per-tool button copy (§7): a skill proposal is an "add", not an "allow". Shared with the
// parked Inbox card so both dialects match.
export function approvalActionLabels(name?: string): { allow: string; deny: string } {
  if (name === "save_skill") return { allow: "Add to my skills", deny: "Not now" };
  if (name === "save_persona") return { allow: "Add this assistant", deny: "Not now" };
  return { allow: "Allow once", deny: "Deny" };
}

// save_skill's review surface (SKILLS-SPEC §5.2): description, the full instructions
// (clamped, expandable, scrollable), every bundled file, and the guaranteed footer that
// answers "added WHERE, available WHEN". Shared verbatim with the parked Inbox card —
// one decision, one dialect.
export function SaveSkillPreview({ args }: { args: any }) {
  return (
    <>
      {args?.description && <div className="approval-with">{String(args.description)}</div>}
      {args?.instructions && <PreviewBlock text={String(args.instructions)} mono={false} />}
      {Array.isArray(args?.files) && args.files.length > 0 && (
        <div data-testid="skill-bundle-files">
          {args.files.map((f: unknown, i: number) => (
            <span className="approval-filechip" key={i}>
              <span className="ico">
                <Icon name="file" size={ICON.md} />
              </span>
              {String(f).split(/[\\/]/).pop() || String(f)}
            </span>
          ))}
        </div>
      )}
      <div className="approval-with">
        Approving adds it to your skills on this computer — usable in every conversation from
        then on.
      </div>
    </>
  );
}

// A `permissions` proposal on the create_scheduled_task consent card (§25): reads are
// disclosure lines, writes are the standing grants the approval mints.
export interface PermissionLine {
  tool: string;
  target: string;
  access: string;
}

export function permissionLines(args: any): PermissionLine[] {
  const raw = args?.permissions;
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((p) => p && typeof p === "object" && p.tool && p.target)
    .map((p) => ({ tool: String(p.tool), target: String(p.target), access: String(p.access || "read") }));
}

// save_persona's review surface: what it is for, what it may use, how far it may act, and
// the standing role itself — the four things that decide what this assistant will do.
export function SavePersonaPreview({ args }: { args: any }) {
  const tools = Array.isArray(args?.tools) ? args.tools : [];
  const skills = Array.isArray(args?.skills) ? args.skills : [];
  return (
    <>
      {args?.tagline && <div className="approval-with">{String(args.tagline)}</div>}
      {args?.instructions && <PreviewBlock text={String(args.instructions)} mono={false} />}
      <div className="approval-with">
        Can use: {tools.length ? tools.map(String).join(", ") : "nothing but the always-on tools"} · Acts:{" "}
        {String(args?.mode || "interactive")} · Skills: {skills.length ? skills.map(String).join(", ") : "whatever is installed"}
      </div>
      <div className="approval-with">
        Approving adds it to your assistants on this computer — pickable in every new conversation.
      </div>
    </>
  );
}

export function TitleText({ line }: { line: HumanLine }) {
  return (
    <span className="approval-title">
      {line.pre}
      {line.obj && <b>{line.obj}</b>}
      {line.post}
    </span>
  );
}

// Plain-words scope note (replaces the "local action" badge): where does this act?
// Shared with the parked-approval card (InboxItemCard) so both dialects match (§35).
export function scopeNote(
  name: string,
  args: any,
  category?: string,
): { text: string; external: boolean } {
  // save_skill's corner answers WHERE (SKILLS-SPEC §5.2): the exact place to find, edit,
  // or turn off the skill afterwards.
  if (name === "save_skill") return { text: "saves to Settings ▸ Skills", external: false };
  if (name === "save_persona") return { text: "adds an assistant on this computer", external: false };
  if (category === "connector") return { text: "acts on a connected service", external: true };
  // The assistant's own browser, signed in to whatever it was signed in to.
  if (category === "browser") return { text: `acts on ${args?.site || "a website"} in its browser`, external: true };
  if (EXTERNAL.has(name)) {
    const platform = String(args?.target ?? "").split(":")[0];
    const names: Record<string, string> = { slack: "Slack", telegram: "Telegram" };
    return { text: `leaves this computer → ${names[platform] || platform || "a connected chat"}`, external: true };
  }
  const overwrite = name === "write_file" && args?.overwrite;
  return { text: "stays on this computer" + (overwrite ? " · overwrites the existing file" : ""), external: false };
}

// The proposed content/command, straight from the tool call's ARGS — the file/action
// doesn't exist yet, so no viewer could show it (§35; see UX-018 mock note).
// Clamps by CHARACTERS as well as lines: a one-paragraph Slack digest has no
// newlines at all and once ballooned the card to full-transcript height.
export const PREVIEW_LINES = 5;
export const PREVIEW_CHARS = 420;

export function PreviewBlock({ text, mono = true }: { text: string; mono?: boolean }) {
  const [all, setAll] = useState(false);
  const lines = text.split("\n");
  const clipped = lines.length > PREVIEW_LINES || text.length > PREVIEW_CHARS;
  let shown = text;
  if (!all && clipped) {
    shown = lines.slice(0, PREVIEW_LINES).join("\n");
    if (shown.length > PREVIEW_CHARS) shown = shown.slice(0, PREVIEW_CHARS).trimEnd() + "…";
  }
  return (
    <div className={"approval-prev" + (mono ? "" : " prose")}>
      {shown}
      {clipped && (
        <button className="approval-prev-more" onClick={() => setAll((v) => !v)}>
          {all
            ? "show less"
            : lines.length > PREVIEW_LINES
              ? `show all ${lines.length} lines`
              : "show the full message"}
        </button>
      )}
    </div>
  );
}

// Outbound message text: short one-liners keep the cozy inline quote; anything
// long (or multi-line) gets the clamped preview so the card stays card-sized.
export function MessagePreview({ text, label }: { text: string; label?: string }) {
  if (text.length <= 220 && !text.includes("\n")) {
    return (
      <div className="approval-with">
        {label ? `${label}: ` : ""}“{text}”
      </div>
    );
  }
  return <PreviewBlock text={text} mono={false} />;
}
