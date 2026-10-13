import type { SessionInfo } from "../../types";

// Command-palette search (Codex-style): clicking Search opens this overlay over the whole app
// rather than filtering the sidebar in place (which made the grouped list collapse). It searches
// ALL sessions, split into Pinned + Recent, filters as you type, and supports ↑/↓ + Enter + ⌘1–9.

export const byRecent = (a: SessionInfo, b: SessionInfo) =>
  (b.updated_at || "").localeCompare(a.updated_at || "");
