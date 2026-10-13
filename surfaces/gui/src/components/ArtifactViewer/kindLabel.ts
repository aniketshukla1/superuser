import { clock } from "../../when";

/** Quiet file-type icons for file lists (the colored kind pills read as noisy). */
export function kindIcon(kind: string): "file" | "fileCode" | "image" | "table" {
  if (kind === "image") return "image";
  if (kind === "html" || kind === "code") return "fileCode";
  if (kind === "csv" || kind === "sheet") return "table";
  return "file"; // markdown, text, pdf, everything else
}

/** Fallback kind for a path the artifact list doesn't know yet — mirrors the server's
 *  extension mapping closely enough for the viewer to pick a renderer. */
export function kindFromPath(path: string): string {
  const ext = (path.split(".").pop() || "").toLowerCase();
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext)) return "image";
  if (["html", "htm"].includes(ext)) return "html";
  if (ext === "md") return "markdown";
  if (ext === "csv") return "csv";
  if (ext === "pdf") return "pdf";
  // `.chart.json`: still valid JSON that anything else can read, and still falls back to
  // the code view when the spec does not parse as a chart.
  if (path.toLowerCase().endsWith(".chart.json")) return "chart";
  if (["py", "js", "ts", "tsx", "jsx", "json", "sh", "css"].includes(ext)) return "code";
  return "text";
}

/** What to call a file kind in the open document's header. `text` is left unnamed —
 *  "Text" next to `notes.txt` is a word that earns nothing. */
export function kindLabel(kind: string): string {
  switch (kind) {
    case "sheet": return "Spreadsheet";
    case "slides": return "Presentation";
    case "doc": return "Document";
    case "office": return "Office document";
    case "markdown": return "Markdown";
    case "csv": return "CSV";
    case "code": return "Code";
    case "image": return "Image";
    case "pdf": return "PDF";
    case "html": return "HTML";
    case "folder": return "Folder";
    default: return "";
  }
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes)) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** A clock time — for a log whose lines are minutes apart, where the day is not the question. */
export function formatTime(epochSeconds: number): string {
  return clock(epochSeconds);
}
