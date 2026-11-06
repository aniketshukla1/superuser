import { baseName, isEmail, messageTarget, onSite, trunc, type HumanLine } from "./humanizeTool";

type Phrase = (a: any) => HumanLine;

const str = (value: unknown, fallback = "") => String(value ?? fallback);
const fileOrFiles = (a: any) => (a.path ? baseName(str(a.path)) : "files");
const asObject = (args: any) => (args !== null && typeof args === "object" ? args : {});

/** The approval card's headline (§35): the request, worded as the action being decided.
 *  run_shell leads with the model's own description ("Run a command — fetch stock data"). */
const TITLES: Record<string, Phrase> = {
  write_file: (a) => ({ pre: "Write ", obj: baseName(str(a.path, "a file")) }),
  replace_in_file: (a) => ({ pre: "Edit ", obj: fileOrFiles(a) }),
  apply_patch: (a) => ({ pre: "Edit ", obj: fileOrFiles(a) }),
  apply_unified_diff: (a) => ({ pre: "Edit ", obj: fileOrFiles(a) }),
  run_shell: (a) => {
    const said = typeof a.description === "string" ? a.description.trim() : "";
    return said ? { pre: "Run a command", post: ` — ${said.charAt(0).toLowerCase()}${said.slice(1)}` } : { pre: "Run a command" };
  },
  send_message: (a) => {
    const { tail } = messageTarget(str(a.target));
    if (!tail) return { pre: "Send a message" };
    return isEmail(a.target) ? { pre: "Send an email to ", obj: tail } : { pre: "Send a message to ", obj: tail };
  },
  send_file: (a) => {
    const { tail } = messageTarget(str(a.target));
    return tail ? { pre: "Send a file to ", obj: tail } : { pre: "Send a file" };
  },
  create_scheduled_task: (a) => (a.title ? { pre: "Create the automation ", obj: `“${trunc(String(a.title), 60)}”` } : { pre: "Create an automation" }),
  // SKILLS-SPEC §5.2 and §7: "Add", never "install", and the destination is "your skills".
  save_skill: (a) => (a.name ? { pre: "Add skill ", obj: String(a.name), post: " to your skills" } : { pre: "Add a skill to your skills" }),
  ask_coworker: (a) => ({ pre: "Ask ", obj: str(a.coworker, "another assistant"), post: ` to ${trunc(str(a.task, "help"), 60)}` }),
  // The same wording SKILLS-SPEC uses for skills, for an assistant: "add", never "install",
  // with the destination said out loud.
  save_persona: (a) => (a.name ? { pre: "Add the assistant ", obj: String(a.name), post: " to your assistants" } : { pre: "Add an assistant" }),
  browser_click: (a) => ({ pre: "Click ", obj: str(a.click, "the page"), post: onSite(a) }),
  browser_type: (a) => ({ pre: "Type into ", obj: str(a.into, "the page"), post: onSite(a) + (a.then ? ` and ${a.then}` : "") }),
  browser_key: (a) => ({ pre: "Press ", obj: str(a.press, "a key"), post: onSite(a) }),
};

/** An approval that never ran (usually declined), worded as what was wanted. */
const ASKS: Record<string, Phrase> = {
  run_shell: (a) => ({ pre: "Wanted to run ", obj: trunc(str(a.command), 60) }),
  write_file: (a) => ({ pre: "Wanted to write ", obj: baseName(str(a.path, "a file")) }),
  replace_in_file: (a) => ({ pre: "Wanted to edit ", obj: fileOrFiles(a) }),
  apply_patch: (a) => ({ pre: "Wanted to edit ", obj: fileOrFiles(a) }),
  apply_unified_diff: (a) => ({ pre: "Wanted to edit ", obj: fileOrFiles(a) }),
  send_message: (a) => {
    const { platform, tail } = messageTarget(str(a.target));
    if (!tail) return { pre: "Wanted to send a message" };
    return isEmail(a.target) ? { pre: "Wanted to email ", obj: tail } : { pre: "Wanted to message ", obj: tail, post: ` on ${platform}` };
  },
  browser_click: (a) => ({ pre: "Wanted to click ", obj: str(a.click, "the page"), post: onSite(a) }),
  browser_type: (a) => ({ pre: "Wanted to type into ", obj: str(a.into, "the page"), post: onSite(a) }),
  browser_key: (a) => ({ pre: "Wanted to press ", obj: str(a.press, "a key"), post: onSite(a) }),
};

const own = (table: Record<string, Phrase>, name: string) => (Object.prototype.hasOwnProperty.call(table, name) ? table[name] : undefined);

export function humanizeApprovalTitle(name: string, args: any): HumanLine {
  return own(TITLES, name)?.(asObject(args)) ?? { pre: `Use ${name}` };
}

export function humanizeAsk(name: string, args: any): HumanLine {
  return own(ASKS, name)?.(asObject(args)) ?? { pre: `Wanted to use ${name}` };
}
