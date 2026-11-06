import type { Attachment } from "./types";

// A dropped or picked File as an Attachment, for the composer and the start panel. Images and
// PDFs travel as data URLs and text files inline; anything else, or anything over 10 MB, is
// left out (null).
const LIMIT = 10 * 1024 * 1024;
const TEXT_EXTENSIONS = new Set([
  "c", "cpp", "css", "csv", "go", "h", "htm", "html", "ini", "java", "js", "json", "jsx", "log",
  "markdown", "md", "py", "rs", "sh", "sql", "toml", "ts", "tsv", "tsx", "txt", "xml", "yaml", "yml",
]);

function extension(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "" : name.slice(dot + 1).toLowerCase();
}

export const isPdfFile = (file: File) => file.type === "application/pdf" || extension(file.name) === "pdf";

type Kind = Attachment["kind"];

function kindOf(file: File): Kind | null {
  if (file.type.startsWith("image/")) return "image";
  if (isPdfFile(file)) return "pdf";
  if (file.type.startsWith("text/") || TEXT_EXTENSIONS.has(extension(file.name))) return "text";
  return null;
}

/** The file's contents as text or as a data URL; null when the read fails. */
function load(file: File, as: "text" | "dataUrl"): Promise<string | null> {
  return new Promise((done) => {
    const fr = new FileReader();
    fr.onload = () => done(String(fr.result));
    fr.onerror = () => done(null);
    if (as === "text") fr.readAsText(file);
    else fr.readAsDataURL(file);
  });
}

export async function readFile(file: File): Promise<Attachment | null> {
  const kind = kindOf(file);
  if (!kind || file.size > LIMIT) return null;
  const body = await load(file, kind === "text" ? "text" : "dataUrl");
  if (body === null) return null;
  switch (kind) {
    case "text":
      return { kind, text: body, mime: file.type, name: file.name || "file.txt" };
    case "pdf":
      return { kind, data_url: body, mime: "application/pdf", name: file.name || "file.pdf" };
    default:
      return { kind: "image", data_url: body, mime: file.type, name: file.name || "image" };
  }
}
