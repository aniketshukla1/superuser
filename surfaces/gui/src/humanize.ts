// UX-015 (§33): tool calls render as English one-liners. The model does NOT emit a purpose
// per call — the stream is name+args+result — so the sentence is synthesized here from
// per-tool templates. `run_shell` is the exception: its optional `description` argument is
// model-written intent and is preferred when present. Fallback: "Used <tool> — <short args>".

export { humanizeTool } from "./humanize/humanizeTool";
export type { HumanLine } from "./humanize/humanizeTool";
export { humanizeApprovalTitle, humanizeAsk } from "./humanize/humanizeApprovalTitle";
