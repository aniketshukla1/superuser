import { fetch, httpBase, jsonBody } from "./neverRejectingJson";
import type { ArtifactContent } from "./Stats";

// Reading and changing the files in a session's folder.

const artifactsUrl = (sessionId: string, action: string) =>
  `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts/${action}`;

/** A write the user is counting on. A non-2xx status or a failed request comes back as
 *  `{ok: false, error}` rather than an empty object, because an empty reply would look the
 *  same as a save that worked, and here the person believes their change is on disk. */
async function save<T extends { ok: boolean; error?: string }>(sessionId: string, action: string, body: unknown): Promise<T> {
  try {
    const res = await fetch(artifactsUrl(sessionId, action), jsonBody("POST", body));
    if (!res.ok) return { ok: false, error: `server said ${res.status}` } as T;
    return await res.json();
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : String(error) } as T;
  }
}

export async function readArtifact(sessionId: string, path: string): Promise<ArtifactContent> {
  const query = new URLSearchParams({ path }).toString();
  return (await fetch(`${artifactsUrl(sessionId, "read")}?${query}`)).json();
}

/** Put a dropped file into the session's folder exactly as it is. Not `writeArtifact`: that is
 *  the text editor's save, and it refuses binary documents, which is right for an editor and
 *  no use here, where a spreadsheet is the typical case. */
export function uploadArtifact(sessionId: string, path: string, dataUrl: string) {
  return save<{ ok: boolean; path?: string; bytes?: number; error?: string }>(sessionId, "upload", { path, data_url: dataUrl });
}

/** Save text back to a file in the session's folder. */
export function writeArtifact(sessionId: string, path: string, content: string) {
  return save<{ ok: boolean; error?: string; bytes?: number }>(sessionId, "write", { path, content });
}

/** Change cells in a workbook through `sheets::write_cells`, the same code the agent's
 *  `sheet_write` uses, so a value typed into the grid means what it would mean from the agent:
 *  `007` stays text and `=SUM(B2:B9)` becomes a formula. `result` is a formula's computed
 *  value; send it whenever there is one, because a formula saved without its cached result is
 *  dropped by any reader that doesn't recalculate, the parser this panel reads with included. */
export function writeSheetCells(sessionId: string, path: string, sheet: string, edits: { cell: string; value: string; result?: string }[]) {
  return save<{ ok: boolean; error?: string; written?: number }>(sessionId, "sheet", { path, sheet, edits });
}

/** Change paragraphs of a .docx. Only the named paragraphs change; styles, tables, images and
 *  every other part of the archive stay as they were (`core::docedit` explains why it isn't
 *  done by regenerating the file). */
export function writeDocParagraphs(sessionId: string, path: string, edits: { index: number; text: string }[]) {
  return save<{ ok: boolean; error?: string; written?: number }>(sessionId, "doc", { path, edits });
}

/** Show the file in the system's file manager ("reveal") or open it in its default app ("open"). */
export async function revealArtifact(sessionId: string, path: string, mode: "reveal" | "open" = "reveal"): Promise<{ ok: boolean; error?: string }> {
  return (await fetch(artifactsUrl(sessionId, "reveal"), jsonBody("POST", { path, mode }))).json();
}
