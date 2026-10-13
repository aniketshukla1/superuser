import { baseName, isEmail, messageTarget, onSite, trunc, type HumanLine } from "./humanizeTool";

// The approval card's headline (§35): the ask, phrased as the action being decided.
// run_shell leads with the model's own description ("Run a command — fetch stock data").
export function humanizeApprovalTitle(name: string, args: any): HumanLine {
  const a = args && typeof args === "object" ? args : {};
  switch (name) {
    case "write_file":
      return { pre: "Write ", obj: baseName(String(a.path ?? "a file")) };
    case "replace_in_file":
    case "apply_patch":
    case "apply_unified_diff":
      return { pre: "Edit ", obj: a.path ? baseName(String(a.path)) : "files" };
    case "run_shell": {
      const desc = typeof a.description === "string" && a.description.trim() ? a.description.trim() : "";
      return {
        pre: "Run a command",
        ...(desc ? { post: ` — ${desc.charAt(0).toLowerCase()}${desc.slice(1)}` } : {}),
      };
    }
    case "send_message": {
      const { tail } = messageTarget(String(a.target ?? ""));
      if (tail && isEmail(a.target)) return { pre: "Send an email to ", obj: tail };
      return tail ? { pre: "Send a message to ", obj: tail } : { pre: "Send a message" };
    }
    case "send_file": {
      const { tail } = messageTarget(String(a.target ?? ""));
      return tail ? { pre: "Send a file to ", obj: tail } : { pre: "Send a file" };
    }
    case "create_scheduled_task":
      return a.title
        ? { pre: "Create the automation ", obj: `“${trunc(String(a.title), 60)}”` }
        : { pre: "Create an automation" };
    case "save_skill":
      // SKILLS-SPEC §5.2/§7: "Add", never "install"; destination is "your skills".
      return a.name
        ? { pre: "Add skill ", obj: String(a.name), post: " to your skills" }
        : { pre: "Add a skill to your skills" };
    case "ask_coworker":
      return { pre: "Ask ", obj: String(a.coworker ?? "another assistant"), post: ` to ${trunc(String(a.task ?? "help"), 60)}` };
    case "save_persona":
      // SKILLS-SPEC's wording for a skill, applied to an assistant: "add", never "install",
      // and the destination said out loud.
      return a.name
        ? { pre: "Add the assistant ", obj: String(a.name), post: " to your assistants" }
        : { pre: "Add an assistant" };
    case "browser_click":
      return { pre: "Click ", obj: String(a.click ?? "the page"), post: onSite(a) };
    case "browser_type":
      return { pre: "Type into ", obj: String(a.into ?? "the page"), post: onSite(a) + (a.then ? ` and ${a.then}` : "") };
    case "browser_key":
      return { pre: "Press ", obj: String(a.press ?? "a key"), post: onSite(a) };
    default:
      return { pre: `Use ${name}` };
  }
}

// Approvals with no executed tool call (typically declined): the ask, phrased as intent.
export function humanizeAsk(name: string, args: any): HumanLine {
  const a = args && typeof args === "object" ? args : {};
  switch (name) {
    case "run_shell":
      return { pre: "Wanted to run ", obj: trunc(String(a.command ?? ""), 60) };
    case "write_file":
      return { pre: "Wanted to write ", obj: baseName(String(a.path ?? "a file")) };
    case "replace_in_file":
    case "apply_patch":
    case "apply_unified_diff":
      return { pre: "Wanted to edit ", obj: a.path ? baseName(String(a.path)) : "files" };
    case "send_message": {
      const { platform, tail } = messageTarget(String(a.target ?? ""));
      if (!tail) return { pre: "Wanted to send a message" };
      if (isEmail(a.target)) return { pre: "Wanted to email ", obj: tail };
      return { pre: `Wanted to message `, obj: tail, post: ` on ${platform}` };
    }
    case "browser_click":
      return { pre: "Wanted to click ", obj: String(a.click ?? "the page"), post: onSite(a) };
    case "browser_type":
      return { pre: "Wanted to type into ", obj: String(a.into ?? "the page"), post: onSite(a) };
    case "browser_key":
      return { pre: "Wanted to press ", obj: String(a.press ?? "a key"), post: onSite(a) };
    default:
      return { pre: `Wanted to use ${name}` };
  }
}
