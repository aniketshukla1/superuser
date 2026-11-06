import { shortArgs } from "../components/ApprovalCard";

/** A step as one sentence in three parts, so the object can be emphasised:
 *  "Read " + <b>runbook.md</b> + " from the shared folder". */
export interface HumanLine {
  pre: string;
  obj?: string;
  post?: string;
}

/** `s` cut to `n` characters, the last one an ellipsis. */
export const trunc = (s: string, n: number) => (s.length <= n ? s : `${s.slice(0, n - 1)}…`);

/** The last part of a slash-separated path, ignoring trailing slashes. */
export function baseName(p: string): string {
  const parts = p.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || p;
}

/** The host of a URL, which may come without a scheme; the input itself if it won't parse. */
export function hostOf(u: string): string {
  try {
    return new URL(u.includes("://") ? u : `https://${u}`).host || u;
  } catch {
    return u;
  }
}

/** " on <site>" for a browser approval. The server writes these arguments for a person (the
 *  control by its label, the site, what is typed) because the model's own, a ref and a
 *  snapshot id, would ask them to approve something they can't see. */
export const onSite = (a: any) => (a.site ? ` on ${a.site}` : "");

const PLATFORM_NAMES: Record<string, string> = { slack: "Slack", telegram: "Telegram" };

/** A send_message target is "platform:chat" or "platform:chat:thread": the platform by name,
 *  and the last readable part of the chat id. */
export function messageTarget(target: string): { platform: string; tail: string } {
  const [platform, chat = ""] = String(target).split(":");
  const tail = chat.includes("/") ? chat.slice(chat.lastIndexOf("/") + 1) || chat : chat;
  return { platform: PLATFORM_NAMES[platform] || platform, tail };
}

/** An email is sent, not "messaged on email", and it takes "an". */
export const isEmail = (target: unknown) => /^(email|mailto):/i.test(String(target ?? ""));

const str = (value: unknown, fallback = "") => String(value ?? fallback);
const quoted = (value: unknown, n: number) => `“${trunc(str(value), n)}”`;
const lowerFirst = (s: string) => s.charAt(0).toLowerCase() + s.slice(1);
const fileOrFiles = (a: any) => (a.path ? baseName(str(a.path)) : "files");
const people = (a: any) => {
  const n = Array.isArray(a.candidates) ? a.candidates.length : 0;
  return n === 1 ? "1 candidate" : `${n || "the"} candidates`;
};

/** How each tool reads in the transcript. A missing entry falls back to "Used <tool>". */
const LINES: Record<string, (a: any) => HumanLine> = {
  run_shell: (a) => {
    const line: HumanLine = { pre: a.run_in_background ? "Started in the background: " : "Ran ", obj: trunc(str(a.command), 60) };
    const said = typeof a.description === "string" ? a.description.trim() : "";
    if (said) line.post = ` — ${lowerFirst(said)}`;
    return line;
  },
  shell_task_output: () => ({ pre: "Checked on a background command" }),
  shell_task_kill: () => ({ pre: "Stopped a background command" }),
  read_file: (a) => ({ pre: "Read ", obj: baseName(str(a.path, "a file")) }),
  write_file: (a) => ({ pre: "Wrote ", obj: baseName(str(a.path, "a file")) }),
  replace_in_file: (a) => ({ pre: "Edited ", obj: fileOrFiles(a) }),
  apply_patch: (a) => ({ pre: "Edited ", obj: fileOrFiles(a) }),
  apply_unified_diff: (a) => ({ pre: "Edited ", obj: fileOrFiles(a) }),
  grep: (a) => ({ pre: "Searched the code for ", obj: quoted(a.pattern, 40) }),
  // Claude Code's file finder, when a turn is handed over to it (core/claude_cli.rs).
  glob: (a) => ({ pre: "Looked for files matching ", obj: quoted(a.pattern, 40) }),
  git_log: () => ({ pre: "Looked through recent git history" }),
  todo_write: (a) => {
    // `todos` is the current key; `items` is how older histories wrote it.
    const list = Array.isArray(a.todos) ? a.todos : Array.isArray(a.items) ? a.items : [];
    if (list.length !== 1) return { pre: `Updated the plan — ${list.length} items` };
    const only = list[0] || {};
    const line: HumanLine = { pre: "Updated the plan — ", obj: quoted(only.content, 70) };
    const status = str(only.status || "").replace(/_/g, " ");
    if (status) line.post = ` → ${status}`;
    return line;
  },
  send_message: (a) => {
    const { platform, tail } = messageTarget(str(a.target));
    if (!tail) return { pre: "Sent a message" };
    return isEmail(a.target) ? { pre: "Sent an email to ", obj: tail } : { pre: `Sent a ${platform} message to `, obj: tail };
  },
  web_search: (a) => ({ pre: "Searched the web — ", obj: quoted(a.query, 60) }),
  web_fetch: (a) => {
    const raw = str(a.url);
    let host = raw;
    try {
      host = new URL(raw).host || raw;
    } catch {
      // Not a full URL: show it as given.
    }
    return { pre: "Read a web page — ", obj: trunc(host, 50) };
  },
  explore: (a) => ({ pre: "Sent a sub-agent to explore — ", obj: quoted(a.task ?? a.prompt, 60) }),
  // SKILLS-SPEC §4.1 #4, the trust line: the moment a skill's instructions were taken up always
  // shows, whether the model chose it or it was forced with /skill.
  load_skill: (a) => ({ pre: "Used skill: ", obj: str(a.name) }),
  ask_user: (a) => {
    // The question itself, not merely that one was asked: the turn stopped here until it was
    // answered, so reading back through the steps should show what was asked.
    const questions = Array.isArray(a.questions) ? a.questions : [a];
    const line: HumanLine = { pre: "Asked you: ", obj: `“${trunc(str(questions[0]?.question ?? a.question), 70)}”` };
    if (questions.length > 1) line.post = ` and ${questions.length - 1} more`;
    return line;
  },
  chart: (a) => ({ pre: "Drew ", obj: str(a.spec?.title, "a chart") }),
  metrics_find: (a) => ({ pre: "Looked for metrics — ", obj: quoted(a.text, 50) }),
  metrics_query: (a) => ({ pre: "Queried Prometheus — ", obj: quoted(a.query, 60) }),
  metrics_range: (a) => ({ pre: "Read a metric over time — ", obj: quoted(a.query, 60) }),
  metrics_compare: (a) => ({ pre: "Compared every series before and after ", obj: str(a.change_at, "the change") }),
  metrics_alerts: () => ({ pre: "Checked the alerts firing in Prometheus" }),
  perf_report: () => ({ pre: "Ran the health report" }),
  candidates_check: (a) => ({ pre: "Checked ", obj: people(a), post: " against your ATS and past shortlists" }),
  candidates_shortlist: (a) => ({ pre: "Shortlisted ", obj: people(a), post: a.role ? ` for ${quoted(a.role, 40)}` : "" }),
  expense_summary: () => ({ pre: "Read the expense ledger" }),
  activity_since: (a) => ({ pre: "Read what came in since ", obj: str(a.since, "yesterday").replace(/_/g, " ") }),
  browser_console: () => ({ pre: "Read what the page reported" }),
  browser_screenshot: () => ({ pre: "Kept a picture of the page" }),
  propose_plan: () => ({ pre: "Proposed a plan" }),
  // The assistant chose to say no and logged it; worded as a choice, which is the whole point.
  report_refusal: (a) => ({ pre: "Declined: ", obj: trunc(str(a.request, "a request"), 60) }),
  request_directory: (a) => ({ pre: "Asked for folder access — ", obj: str(a.path) }),
  // The assistant's own browser. A click's arguments are a ref into a snapshot, meaningless off
  // the page, so the line says what kind of thing happened and the step's result says what.
  browser_navigate: (a) => ({ pre: "Opened ", obj: trunc(hostOf(str(a.url)), 50), post: " in its browser" }),
  browser_read: () => ({ pre: "Read the page in its browser" }),
  browser_snapshot: () => ({ pre: "Looked at what's on the page" }),
  browser_click: () => ({ pre: "Clicked on the page" }),
  browser_type: (a) => {
    const typed = str(a.text);
    const line: HumanLine = { pre: "Typed ", obj: typed ? `“${trunc(typed, 50)}”` : "into a field" };
    if (a.submit) line.post = " and pressed Enter";
    return line;
  },
  browser_key: (a) => ({ pre: "Pressed ", obj: str(a.key, "a key") }),
  browser_scroll: (a) => ({ pre: a.direction === "up" ? "Scrolled up" : "Scrolled down" }),
  browser_request_help: (a) => ({ pre: "Asked for your hands — ", obj: trunc(str(a.reason), 70) }),
  browser_request_secret: (a) => ({ pre: "Asked you to enter ", obj: trunc(str(a.label, "a value"), 50) }),
  list_skills: () => ({ pre: "Looked at the skills on this machine" }),
  read_skill: (a) => ({ pre: "Read the skill ", obj: str(a.name) }),
  list_skill_tools: () => ({ pre: "Checked which tools a skill may declare" }),
  save_skill: (a) => ({ pre: "Proposed the skill ", obj: str(a.name), post: " for you to approve" }),
  list_personas: () => ({ pre: "Looked at the assistants on this machine" }),
  read_persona: (a) => ({ pre: "Read the assistant ", obj: str(a.id) }),
  list_persona_skills: () => ({ pre: "Checked which skills an assistant can be given" }),
  save_persona: (a) => ({ pre: "Proposed the assistant ", obj: str(a.name ?? a.id), post: " for you to approve" }),
  ask_coworker: (a) => {
    const task = trunc(str(a.task), 60);
    const line: HumanLine = { pre: "Asked ", obj: str(a.coworker, "another assistant") };
    if (task) line.post = ` — ${task}`;
    return line;
  },
};

/** A tool call as a plain-English line for the transcript. */
export function humanizeTool(name: string, args: any): HumanLine {
  const a = args !== null && typeof args === "object" ? args : {};
  const known = Object.prototype.hasOwnProperty.call(LINES, name) ? LINES[name] : undefined;
  if (known) return known(a);
  const line: HumanLine = { pre: `Used ${name}` };
  const rest = trunc(shortArgs(a), 80);
  if (rest) line.post = ` — ${rest}`;
  return line;
}
