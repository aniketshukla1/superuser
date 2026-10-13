import { useRef, useState } from "react";
import type { Attachment } from "../../types";
import { isPdfFile, readFile } from "../../attach";
import { getSettings, inspectPdf, uploadArtifact } from "../../api";
import { analysePrompt, isDataFile, shouldAutoAnalyse } from "../../dropAnalyse";
import { mergeAttachments } from "./Props";

/** Files on the way into a message: reading them, the PDF limits, and the dropped-table
 * shortcut that starts an analysis instead of attaching. */
export function useAttachments({
  text,
  sessionId,
  running,
  onSend,
}: {
  text: string;
  sessionId?: string;
  running?: boolean;
  onSend: (text: string, attachments: Attachment[], skill?: string) => void;
}) {
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [attachNotice, setAttachNotice] = useState<string | null>(null);
  const noticeTimer = useRef<number | null>(null);

  // Rejected-attachment notice: visible ~8s, then clears (or on ✕).
  const showAttachNotice = (message: string) => {
    setAttachNotice(message);
    if (noticeTimer.current) window.clearTimeout(noticeTimer.current);
    noticeTimer.current = window.setTimeout(() => setAttachNotice(null), 8000);
  };

  const addFiles = async (files: FileList | File[]) => {
    const list = Array.from(files);
    let maxPages = 20;
    let maxMb = 10;
    if (list.some(isPdfFile)) {
      try {
        const s = await getSettings();
        if (s.pdf_max_pages) maxPages = s.pdf_max_pages;
        if (s.pdf_max_mb) maxMb = s.pdf_max_mb;
      } catch {
        /* offline settings fetch — fall back to defaults */
      }
    }
    const accepted: File[] = [];
    for (const file of list) {
      if (isPdfFile(file) && file.size > maxMb * 1024 * 1024) {
        showAttachNotice(
          `${file.name} skipped — ${(file.size / 1024 / 1024).toFixed(1)} MB is over your ${maxMb} MB limit (Settings → Token savings)`,
        );
        continue;
      }
      accepted.push(file);
    }
    const read = (await Promise.all(accepted.map(readFile))).filter(Boolean) as Attachment[];
    const next: Attachment[] = [];
    for (const a of read) {
      if (a.kind === "pdf" && a.data_url) {
        const info = await inspectPdf(a.data_url).catch(() => null);
        if (info?.ok && (info.pages ?? 0) > maxPages) {
          showAttachNotice(
            `${a.name} skipped — ${info.pages} pages is over your ${maxPages}-page limit (Settings → Token savings)`,
          );
          continue;
        }
        if (info && !info.ok) {
          showAttachNotice(`${a.name} skipped — ${info.error || "could not read PDF"}`);
          continue;
        }
      }
      next.push(a);
    }
    if (!next.length) return;

    // A dropped table with nothing typed is an instruction, not an attachment: it goes into
    // the session's folder and starts the analysis. With something typed it just attaches —
    // taking a half-written question and sending something else instead is a worse failure
    // than making someone press Enter. See dropAnalyse.ts for the rule.
    const names = accepted.map((f) => f.name);
    if (sessionId && shouldAutoAnalyse({ names, draft: text, running: !!running })) {
      const tables = accepted.filter((f) => isDataFile(f.name));
      const saved: string[] = [];
      for (const f of tables) {
        // Read as a data URL whatever the type: a .xlsx is bytes, and the upload endpoint
        // takes bytes precisely so it does not have to be text.
        const url = await new Promise<string | null>((res) => {
          const r = new FileReader();
          r.onload = () => res(String(r.result));
          r.onerror = () => res(null);
          r.readAsDataURL(f);
        });
        if (!url) continue;
        const out = await uploadArtifact(sessionId, f.name, url);
        if (out.ok) saved.push(f.name);
        else showAttachNotice(`${f.name} could not be saved — ${out.error || "unknown error"}`);
      }
      if (saved.length) {
        // The file is on disk now, so it is not also an attachment: sending both would put
        // the same table in the context twice, once as a path and once as its contents.
        onSend(analysePrompt(saved), [], undefined);
        return;
      }
    }

    setAttachments((a) => mergeAttachments(a, next));
  };

  return { attachments, setAttachments, attachNotice, setAttachNotice, addFiles };
}
