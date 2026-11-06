// Tool calls shown as plain English. The stream only carries a tool's name, arguments and
// result, so each sentence is built here from a template per tool. `run_shell` may bring its
// own `description`, written by the model, and that wins when present. A tool without a
// template reads "Used <tool> — <short args>".

export { humanizeApprovalTitle, humanizeAsk } from "./humanize/humanizeApprovalTitle";
export type { HumanLine } from "./humanize/humanizeTool";
export { humanizeTool } from "./humanize/humanizeTool";
