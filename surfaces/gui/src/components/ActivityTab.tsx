// The Activity tab — what the run actually did, at an altitude you can read.
//
// This is the run journal (/v1/sessions/:id/run) rendered as work rather than as
// frames. The journal emits `tool_proposed` and `tool_finished` as separate lines;
// a tool call is only legible once those two halves are back together, with the
// arguments as the title and the result folded away until asked for.
//
// The tab is split in two, because the two readings answer different questions and
// you usually want both at once. The top half is the plan and the tool calls — what
// the agent decided and what it reached for. The bottom half is the raw frame feed,
// unabridged: when something surprising happens it is still the honest answer, and
// it is no use as an answer if you have to go looking for it. It used to sit behind
// a collapsed toggle underneath the whole call list, which meant that in a long run
// it was effectively invisible, and even once opened every line was clipped to one
// ellipsised row. Both halves scroll on their own and the divider between them drags.

export { toolCallsFrom, BROWSER_OPEN_TOOL, browserUrlWantedBy, shouldSurfaceBrowser, MANUAL_TAB_GRACE_MS, filesTouchedBy } from "./ActivityTab/ToolRow";
export type { ToolCall } from "./ActivityTab/ToolRow";
export { clampSplit } from "./ActivityTab/SplitHandle";
export { ActivityTab, rawText } from "./ActivityTab/ActivityTab";
