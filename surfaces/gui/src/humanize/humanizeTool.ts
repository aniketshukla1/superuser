import { shortArgs } from "../components/ApprovalCard";

// A one-line sentence in three segments so the UI can emphasize the object:
// "Read " + <b>runbook.md</b> + " from the shared folder".
export interface HumanLine {
  pre: string;
  obj?: string;
  post?: string;
}

export const trunc = (s: string, n: number) => (s.length > n ? s.slice(0, n - 1) + "…" : s);
export const baseName = (p: string) => p.replace(/\/+$/, "").split("/").pop() || p;
export const hostOf = (u: string) => {
  try {
    return new URL(u.includes("://") ? u : `https://${u}`).host || u;
  } catch {
    return u;
  }
};

// A browser approval's arguments are written for a person by the server — the control by
// its label, the site, what goes in — because the model's own (a ref and a snapshot id)
// would ask them to approve something they cannot see.
export const onSite = (a: any) => (a.site ? ` on ${a.site}` : "");

// send_message targets are "platform:chat" or "platform:chat:thread" — show the platform
// by name and the last human-ish segment of the chat id.
export function messageTarget(target: string): { platform: string; tail: string } {
  const [platform, ...rest] = String(target).split(":");
  const chat = rest[0] || "";
  const tail = chat.includes("/") ? chat.split("/").pop() || chat : chat;
  const names: Record<string, string> = { slack: "Slack", telegram: "Telegram" };
  return { platform: names[platform] || platform, tail };
}

// An email is sent, not "messaged on email" — and it takes "an".
export const isEmail = (target: unknown) => /^(email|mailto):/i.test(String(target ?? ""));

export function humanizeTool(name: string, args: any): HumanLine {
  const a = args && typeof args === "object" ? args : {};
  switch (name) {
    case "run_shell": {
      const cmd = trunc(String(a.command ?? ""), 60);
      const desc = typeof a.description === "string" && a.description.trim() ? a.description.trim() : "";
      const pre = a.run_in_background ? "Started in the background: " : "Ran ";
      return {
        pre,
        obj: cmd,
        ...(desc ? { post: ` — ${desc.charAt(0).toLowerCase()}${desc.slice(1)}` } : {}),
      };
    }
    case "shell_task_output":
      return { pre: "Checked on a background command" };
    case "shell_task_kill":
      return { pre: "Stopped a background command" };
    case "read_file":
      return { pre: "Read ", obj: baseName(String(a.path ?? "a file")) };
    case "write_file":
      return { pre: "Wrote ", obj: baseName(String(a.path ?? "a file")) };
    case "replace_in_file":
    case "apply_patch":
    case "apply_unified_diff":
      return { pre: "Edited ", obj: a.path ? baseName(String(a.path)) : "files" };
    case "grep":
      return { pre: "Searched the code for ", obj: `“${trunc(String(a.pattern ?? ""), 40)}”` };
    // Claude Code's file finder, when a turn is handed to it (core/claude_cli.rs).
    case "glob":
      return { pre: "Looked for files matching ", obj: `“${trunc(String(a.pattern ?? ""), 40)}”` };
    case "git_log":
      return { pre: "Looked through recent git history" };
    case "todo_write": {
      // `todos` is current; `items` renders histories from before the rename (the old
      // key breaks Together's GLM-5.2 chat template — see coworker/tools/todo.py).
      const items = Array.isArray(a.todos) ? a.todos : Array.isArray(a.items) ? a.items : [];
      if (items.length === 1) {
        const it = items[0] || {};
        const status = String(it.status || "").replace(/_/g, " ");
        return {
          pre: "Updated the plan — ",
          obj: `“${trunc(String(it.content ?? ""), 70)}”`,
          ...(status ? { post: ` → ${status}` } : {}),
        };
      }
      return { pre: `Updated the plan — ${items.length} items` };
    }
    case "send_message": {
      const { platform, tail } = messageTarget(String(a.target ?? ""));
      if (!tail) return { pre: "Sent a message" };
      if (isEmail(a.target)) return { pre: "Sent an email to ", obj: tail };
      return { pre: `Sent a ${platform} message to `, obj: tail };
    }
    case "web_search":
      return { pre: "Searched the web — ", obj: `“${trunc(String(a.query ?? ""), 60)}”` };
    case "web_fetch": {
      let host = String(a.url ?? "");
      try {
        host = new URL(host).host || host;
      } catch {
        /* keep raw */
      }
      return { pre: "Read a web page — ", obj: trunc(host, 50) };
    }
    case "explore":
      return { pre: "Sent a sub-agent to explore — ", obj: `“${trunc(String(a.task ?? a.prompt ?? ""), 60)}”` };
    case "load_skill":
      // SKILLS-SPEC §4.1 #4 — the trust line: the transcript always shows the moment a
      // skill's instructions were picked up, model-invoked or forced via /skill.
      return { pre: "Used skill: ", obj: String(a.name ?? "") };
    case "ask_user": {
      // The question itself, not just that one happened. This row is now the record
      // of a real exchange — the turn stopped here until you answered — so reading
      // back through the steps should show what was asked, the way the answer shows
      // what was said.
      const qs = Array.isArray(a.questions) ? a.questions : [a];
      const first = String(qs[0]?.question ?? a.question ?? "");
      return {
        pre: "Asked you: ",
        obj: `“${trunc(first, 70)}”`,
        ...(qs.length > 1 ? { post: ` and ${qs.length - 1} more` } : {}),
      };
    }
    case "chart":
      return { pre: "Drew ", obj: String(a.spec?.title ?? "a chart") };
    case "propose_plan":
      return { pre: "Proposed a plan" };
    case "report_refusal":
      // The assistant chose to say no, and noted it in the trail. Worded as a choice,
      // because that is the whole distinction the tool exists to record.
      return { pre: "Declined: ", obj: trunc(String(a.request ?? "a request"), 60) };
    case "request_directory":
      return { pre: "Asked for folder access — ", obj: String(a.path ?? "") };
    // The assistant's own browser. A click's args are a ref into a snapshot, which means
    // nothing off the page — so the row says what kind of thing happened, and the step's
    // result (one click away) says what was clicked.
    case "browser_navigate":
      return { pre: "Opened ", obj: trunc(hostOf(String(a.url ?? "")), 50), post: " in its browser" };
    case "browser_read":
      return { pre: "Read the page in its browser" };
    case "browser_snapshot":
      return { pre: "Looked at what's on the page" };
    case "browser_click":
      return { pre: "Clicked on the page" };
    case "browser_type": {
      const text = String(a.text ?? "");
      return {
        pre: "Typed ",
        obj: text ? `“${trunc(text, 50)}”` : "into a field",
        ...(a.submit ? { post: " and pressed Enter" } : {}),
      };
    }
    case "browser_key":
      return { pre: "Pressed ", obj: String(a.key ?? "a key") };
    case "browser_scroll":
      return { pre: a.direction === "up" ? "Scrolled up" : "Scrolled down" };
    case "list_skills":
      return { pre: "Looked at the skills on this machine" };
    case "read_skill":
      return { pre: "Read the skill ", obj: String(a.name ?? "") };
    case "list_skill_tools":
      return { pre: "Checked which tools a skill may declare" };
    case "save_skill":
      return { pre: "Proposed the skill ", obj: String(a.name ?? ""), post: " for you to approve" };
    case "list_personas":
      return { pre: "Looked at the assistants on this machine" };
    case "read_persona":
      return { pre: "Read the assistant ", obj: String(a.id ?? "") };
    case "list_persona_skills":
      return { pre: "Checked which skills an assistant can be given" };
    case "save_persona":
      return { pre: "Proposed the assistant ", obj: String(a.name ?? a.id ?? ""), post: " for you to approve" };
    case "ask_coworker": {
      const task = trunc(String(a.task ?? ""), 60);
      return { pre: "Asked ", obj: String(a.coworker ?? "another assistant"), ...(task ? { post: ` — ${task}` } : {}) };
    }
    case "browser_request_help":
      return { pre: "Asked for your hands — ", obj: trunc(String(a.reason ?? ""), 70) };
    case "browser_request_secret":
      return { pre: "Asked you to enter ", obj: trunc(String(a.label ?? "a value"), 50) };
    default: {
      const rest = trunc(shortArgs(a), 80);
      return { pre: `Used ${name}`, ...(rest ? { post: ` — ${rest}` } : {}) };
    }
  }
}
