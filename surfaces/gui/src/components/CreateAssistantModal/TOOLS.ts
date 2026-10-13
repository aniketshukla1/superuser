// "New assistant" — name it, say what it's responsible for, hand it reference files,
// and choose how much rope it gets.
//
// The responsibilities text becomes the assistant's system prompt AND its self-
// introduction, so what you type is literally what it will say it does — nothing is
// invented on your behalf. Everything lands in data_dir/personas/<id>.md (plus an
// <id>-files/ folder), plain files you can edit or delete by hand.

export const ICONS = ["sparkles", "code", "wrench", "book", "inbox", "audit"] as const;

export const MODES: { id: string; label: string; hint: string }[] = [
  { id: "interactive", label: "Ask first", hint: "Acts, but asks before anything that sends, writes or deletes" },
  { id: "auto", label: "Work freely", hint: "Acts without asking — only for work you'd let run unattended" },
  { id: "plan", label: "Plan only", hint: "Writes a plan and stops; never acts" },
  { id: "discuss", label: "Discuss only", hint: "Talks it through; no tools, no actions" },
];

// Everything an assistant can be granted. This offered six of the app's fifteen
// capabilities, so an assistant you made could never be given the knowledge base,
// scheduling, subagents or messaging — the built-in assistant has three of those.
// Asking a question and proposing a plan are not here because they are not
// grantable: every assistant has them.
export const TOOLS: { id: string; label: string; hint: string }[] = [
  { id: "recall_memory", label: "memory", hint: "Recall what it knows about you" },
  { id: "web", label: "web", hint: "Search and read the web" },
  { id: "files", label: "files", hint: "Read and write files, and edit spreadsheets" },
  { id: "search", label: "search", hint: "Search across the workspace" },
  { id: "todo", label: "todo", hint: "Plan multi-step work (drives the Progress panel)" },
  { id: "knowledge", label: "knowledge", hint: "Keep topic pages it can look things up in" },
  { id: "connectors", label: "connectors", hint: "Use your connected apps" },
  { id: "schedule", label: "scheduling", hint: "Set up reminders and recurring work" },
  { id: "subagent", label: "subagents", hint: "Delegate focused subtasks" },
  { id: "git", label: "git", hint: "Inspect repository state and history" },
  { id: "messaging", label: "messaging", hint: "Send messages as you — always approval-gated" },
  { id: "shell", label: "shell", hint: "Run shell commands — powerful; grant deliberately" },
  { id: "computer", label: "computer", hint: "Control the screen and keyboard — grant deliberately" },
];

export const DEFAULT_TOOLS = ["recall_memory", "web", "files", "todo", "connectors"];

// Reference files are stored as text so the assistant can actually read them; a
// binary blob it can't parse would be a lie, so we refuse those by extension.
export const TEXTY = /\.(md|markdown|txt|csv|tsv|json|ya?ml|toml|ini|log|html?|xml|ts|tsx|js|jsx|py|rs|go|java|rb|sh|sql)$/i;

export type Picked = { name: string; content: string };
