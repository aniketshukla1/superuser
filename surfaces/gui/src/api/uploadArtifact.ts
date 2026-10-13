import { fetch, httpBase } from "./neverRejectingJson";
import type { ArtifactContent } from "./Stats";

export async function readArtifact(sessionId: string, path: string): Promise<ArtifactContent> {
  const q = new URLSearchParams({ path });
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts/read?${q.toString()}`);
  return res.json();
}

/**
 * Save text back to a file in the session's folder.
 *
 * Deliberately not routed through `neverRejectingJson`: a failed save that returns
 * `{}` is indistinguishable from a successful one, and this is the one call where
 * the user believes their edit is on disk. The server always answers with `ok`.
 */
/** Put a dropped file into the session's folder, unaltered.
 *
 * Not `writeArtifact`: that is the text editor's save and refuses binary documents, which is
 * right for it and useless here — a spreadsheet is the case this exists for. */
export async function uploadArtifact(
  sessionId: string,
  path: string,
  dataUrl: string,
): Promise<{ ok: boolean; path?: string; bytes?: number; error?: string }> {
  try {
    const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts/upload`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, data_url: dataUrl }),
    });
    if (!res.ok) return { ok: false, error: `server said ${res.status}` };
    return await res.json();
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

export async function writeArtifact(
  sessionId: string,
  path: string,
  content: string,
): Promise<{ ok: boolean; error?: string; bytes?: number }> {
  try {
    const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts/write`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, content }),
    });
    if (!res.ok) return { ok: false, error: `server said ${res.status}` };
    return await res.json();
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/**
 * Edit cells in a workbook. Goes through the same `sheets::write_cells` the agent's
 * `sheet_write` tool uses, so a value typed in the grid is interpreted exactly as one
 * the agent writes: `007` stays text, `=SUM(B2:B9)` becomes a formula.
 *
 * `result` is a formula's computed value. Send it whenever there is one: a formula
 * stored without its cached result is dropped by every reader that doesn't
 * recalculate — including the parser this very panel uses to read the file back.
 */
export async function writeSheetCells(
  sessionId: string,
  path: string,
  sheet: string,
  edits: { cell: string; value: string; result?: string }[],
): Promise<{ ok: boolean; error?: string; written?: number }> {
  try {
    const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts/sheet`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, sheet, edits }),
    });
    if (!res.ok) return { ok: false, error: `server said ${res.status}` };
    return await res.json();
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/**
 * Edit paragraphs of a .docx. Only the paragraphs named change; the rest of the
 * document — styles, tables, images, and every other part of the archive — is left
 * exactly as it was. See `core::docedit` for why it is done this way rather than by
 * regenerating the file.
 */
export async function writeDocParagraphs(
  sessionId: string,
  path: string,
  edits: { index: number; text: string }[],
): Promise<{ ok: boolean; error?: string; written?: number }> {
  try {
    const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts/doc`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, edits }),
    });
    if (!res.ok) return { ok: false, error: `server said ${res.status}` };
    return await res.json();
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/** Show the artifact in the OS file manager ("reveal") or open it with its default app ("open"). */
export async function revealArtifact(
  sessionId: string,
  path: string,
  mode: "reveal" | "open" = "reveal",
): Promise<{ ok: boolean; error?: string }> {
  const res = await fetch(`${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/artifacts/reveal`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path, mode }),
  });
  return res.json();
}
