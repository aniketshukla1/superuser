import { useEffect, useRef, useState } from "react";
import { writeArtifact, writeSheetCells } from "../../api";
import { decodeDataUrl, toCsv, type Instance } from "./externalChangeAction";
import { editsFromChange, workbookToSnapshot } from "./snapshot";
import { useExternalChange } from "./useExternalChange";

export function UniverSheet({
  dataUrl,
  sessionId,
  path,
  format = "xlsx",
}: {
  dataUrl: string;
  sessionId: string;
  path: string;
  /**
   * A .csv is a grid too — it was rendered as a static table and edited as raw text,
   * which is a worse tool for the job than the one already sitting next to it. The
   * only real difference is the save: a workbook takes cell edits, a CSV is rewritten
   * whole, because it has no cells to address — just text that happens to be tabular.
   */
  format?: "xlsx" | "csv";
  /**
   * Intentionally unused. The obvious thing — re-read the file after each save, as
   * the old grid did — is wrong here: re-reading changes `dataUrl`, which re-runs
   * the effect below and rebuilds the whole editor, throwing away selection and
   * undo history on every keystroke. While the editor is open **it** is the source
   * of truth; the file is where that truth is persisted.
   */
  onSaved?: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  // Saves are serialised through this queue. `write_cells` is read-modify-write —
  // it loads the whole workbook, applies the edits and writes it back — so two
  // requests in flight at once both read the *original* file and the second one
  // silently drops the first one's edit. Measured: setting C1 and B5 in the same
  // tick persisted only B5. Coalescing by cell also collapses a burst of typing
  // into one write instead of one per keystroke.
  const pending = useRef(new Map<string, { sheet: string; cell: string; value: string; result?: string }>());
  const flushing = useRef(false);
  const [error, setError] = useState("");
  const [status, setStatus] = useState("Loading the editor…");
  const { loadedUrl, stale, reload } = useExternalChange(dataUrl, host);
  // Saves are fire-and-forget per edit, but a failure has to be visible: the user
  // believes a typed cell is on disk.
  const [saveError, setSaveError] = useState("");

  useEffect(() => {
    let dead = false;
    let instance: Instance | null = null;

    (async () => {
      try {
        const [{ createUniver, LocaleType, mergeLocales }, corePreset, enUS, XLSX] = await Promise.all([
          import("@univerjs/presets"),
          import("@univerjs/preset-sheets-core"),
          import("@univerjs/preset-sheets-core/locales/en-US"),
          import("xlsx"),
        ]);
        await import("@univerjs/preset-sheets-core/lib/index.css");
        if (dead || !host.current) return;

        // ── xlsx → Univer snapshot ──────────────────────────────────────────
        const wb =
          format === "csv"
            ? XLSX.read(decodeDataUrl(loadedUrl), { type: "string" })
            : XLSX.read(loadedUrl.split(",")[1] || "", { type: "base64" });
        const { sheets, sheetOrder } = workbookToSnapshot(XLSX, wb);

        const { univer, univerAPI } = createUniver({
          locale: LocaleType.EN_US,
          locales: { [LocaleType.EN_US]: mergeLocales(enUS.default) },
          presets: [corePreset.UniverSheetsCorePreset({ container: host.current })],
        });
        instance = { dispose: () => univer.dispose() };
        // A canvas editor can't be driven by querySelector, so development builds
        // expose the facade for debugging and for verifying the save path.
        if (import.meta.env.DEV) {
          (window as unknown as { __univerAPI?: unknown }).__univerAPI = univerAPI;
        }

        univerAPI.createWorkbook({
          id: "workbook",
          name: path.split("/").pop() || "Workbook",
          sheetOrder,
          sheets: sheets as never,
        } as never);

        // ── edits → disk ────────────────────────────────────────────────────
        // Every changed cell goes through the same `sheets::write_cells` the agent's
        // `sheet_write` tool uses, so a value typed here is interpreted identically
        // and the untouched parts of the workbook survive.
        univerAPI.addEvent(univerAPI.Event.SheetValueChanged, async (params: unknown) => {
          const { edits, sheetName } = editsFromChange(params);
          if (!edits.length) return;
          if (format === "csv") {
            // No cell addressing to save into — serialise the sheet and replace the
            // file. Debounced through the same queue so a burst of typing is one write.
            pending.current.set("csv", { sheet: "", cell: "", value: "" });
            void flush();
            return;
          }
          for (const e of edits) pending.current.set(`${sheetName}!${e.cell}`, { sheet: sheetName, ...e });
          void flush();
        });

        async function flush() {
          if (flushing.current) return;
          flushing.current = true;
          try {
            while (pending.current.size) {
              const batch = [...pending.current.values()];
              pending.current.clear();
              if (format === "csv") {
                // The workbook is gone if the editor was disposed mid-flush.
                const book = univerAPI.getActiveWorkbook();
                if (!book) return;
                const rows = (book.getActiveSheet().getDataRange().getValues() ?? []) as unknown[][];
                const res = await writeArtifact(sessionId, path, toCsv(rows));
                if (dead) return;
                if (!res.ok) {
                  setSaveError(res.error || "couldn't save");
                  return;
                }
                setSaveError("");
                continue;
              }
              const bySheet = new Map<string, { cell: string; value: string; result?: string }[]>();
              for (const e of batch) {
                const list = bySheet.get(e.sheet) ?? [];
                list.push({ cell: e.cell, value: e.value, result: e.result });
                bySheet.set(e.sheet, list);
              }
              for (const [sheet, cells] of bySheet) {
                const res = await writeSheetCells(sessionId, path, sheet, cells);
                if (dead) return;
                if (!res.ok) {
                  setSaveError(res.error || "couldn't save");
                  return;
                }
              }
              setSaveError("");
            }
          } finally {
            flushing.current = false;
          }
        }

        if (!dead) setStatus("");
      } catch (e) {
        if (!dead) setError(e instanceof Error ? e.message : String(e));
      }
    })();

    return () => {
      dead = true;
      // React 18 mounts effects twice in development; disposing here is what stops a
      // second Univer instance rendering on top of the first.
      try {
        instance?.dispose();
      } catch {
        /* already gone */
      }
    };
  }, [loadedUrl, sessionId, path, format]);

  return (
    <div className="univer-wrap">
      {error && <div className="rail-error univer-note">Couldn't open the editor: {error}</div>}
      {!error && status && <div className="rail-muted univer-note">{status}</div>}
      {saveError && <div className="rail-error univer-note">{saveError}</div>}
      {stale && (
        <div className="univer-note univer-stale">
          <span>This file changed on disk.</span>
          <button
            className="artifact-text-btn primary"
            onClick={reload}
          >
            Reload
          </button>
        </div>
      )}
      <div className="univer-host" ref={host} />
    </div>
  );
}
