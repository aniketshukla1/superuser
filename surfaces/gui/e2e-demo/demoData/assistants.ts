import { fromSource } from "./FromSource";

// ---- assistants: the real built-ins, with a handful surfaced in the sidebar ----
export const SURFACED = ["assistant", "chief", "perf", "talent", "repro", "email", "expenses", "coder"];
export const PERSONAS = fromSource.personas.map((p) => ({
  id: p.id,
  name: p.name,
  icon: p.icon,
  tagline: p.tagline,
  needs_workspace: p.tools.includes("files") || p.tools.includes("shell"),
  builtin: true,
  family: p.family,
  workspace: p.id === "coder" ? "git" : "deliverable",
  tools: p.tools,
  enabled: true,
  surfaced: SURFACED.includes(p.id),
  default: p.id === "assistant",
}));
