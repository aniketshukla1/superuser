import { fetch, httpBase, jsonOf, listOf } from "./neverRejectingJson";

// ---- Memory Tree ---------------------------------------------------------------

export interface MemoryRow {
  id: string;
  filename: string;
  title: string;
  category: string;
  tags: string[];
  summary: string;
  date: string;
  weight: number;
  body?: string;
  /** Who wrote the content: "internal" (you, or the app) or "external" (a synced
   *  app, an MCP server, the web). Absent on a core too old to send it — treated
   *  as external, the same way the core treats an unmarked file. */
  origin?: "internal" | "external";
}

/** Saved memories, newest first. `q` filters across title, summary, category, tags and body. */
export async function listMemories(q?: string): Promise<MemoryRow[]> {
  const qs = q?.trim() ? `?q=${encodeURIComponent(q.trim())}` : "";
  const res = await fetch(`${httpBase()}/v1/memory${qs}`);
  return listOf(res, "memories");
}

/** One memory including its body — the list omits bodies to stay small. */
export async function getMemory(filename: string): Promise<MemoryRow | null> {
  const res = await fetch(`${httpBase()}/v1/memory/${encodeURIComponent(filename)}`);
  const j = await jsonOf(res);
  return j.ok ? j.memory : null;
}

/** Forget one. A memory store you can't correct is one you can't trust. */
export async function deleteMemory(filename: string): Promise<boolean> {
  const res = await fetch(`${httpBase()}/v1/memory/${encodeURIComponent(filename)}`, { method: "DELETE" });
  return (await jsonOf(res)).ok === true;
}
