import { useEffect, useState } from "react";
import { getSettings, setPdfSettings, type PdfSettings } from "../../api";
import { reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";

// Telemetry/Privacy card removed for this release (owner ask 2026-07-22); the
// setCloudTelemetry API stays for a future opt-out surface.

// -- Sidebar density -------------------------------------------------------------
// -- Token savings (PDF attachments; owner ask, 2026-07-17) ---------------------
// Attachments replay with EVERY turn, so a big PDF quietly multiplies token spend.
// This card is the attachment dial: attach thresholds + the fallback for models
// without native PDF support. (Long-history spend is handled by auto-compaction —
// the CompactionCard below, OPE-27.)
export function TokenSavingsCard() {
  const [pdf, setPdf] = useState<PdfSettings | null>(null);

  useEffect(() => {
    getSettings()
      .then((s) =>
        setPdf({
          pdf_fallback: s.pdf_fallback || "text",
          pdf_max_pages: s.pdf_max_pages || 20,
          pdf_max_mb: s.pdf_max_mb || 10,
        }),
      )
      .catch(() => setPdf({ pdf_fallback: "text", pdf_max_pages: 20, pdf_max_mb: 10 }));
  }, []);

  const save = async (patch: Partial<PdfSettings>) => {
    const before = pdf;
    setPdf((p) => (p ? { ...p, ...patch } : p));
    // A refused write used to leave the control showing the new value while the
    // server kept the old one (UX-REPORT §5). Put it back, and say so.
    const saved = await reportIfFailed(setPdfSettings(patch), "Couldn't save your token-savings settings.");
    if (!saved) setPdf(before);
  };

  if (!pdf) return null;
  return (
    <div className={CARD + " p-4 mb-4"} data-testid="token-savings-card">
      <div className={FIELD_LABEL}>Token savings</div>
      <div className={FIELD_HELP}>
        PDF attachments travel with every turn of a conversation, so large documents multiply
        what you spend on tokens.
      </div>

      <div className="mt-3 text-md text-ink">PDFs on models without native PDF support</div>
      <div className="seg mt-2" role="radiogroup" aria-label="PDF fallback" data-testid="pdf-fallback">
        <button
          className={pdf.pdf_fallback === "text" ? "active" : ""}
          onClick={() => save({ pdf_fallback: "text" })}
        >
          Extract text
        </button>
        <button
          className={pdf.pdf_fallback === "images" ? "active" : ""}
          onClick={() => save({ pdf_fallback: "images" })}
        >
          Send page images
        </button>
      </div>
      <div className={FIELD_HELP}>
        Claude, GPT and Gemini read PDFs natively — this only applies to models that
        don&rsquo;t (GLM, Kimi, DeepSeek, local models…). Text extraction is cheapest; page
        images cost more tokens and need a vision-capable model.
      </div>

      <div className="mt-3 flex items-center gap-5">
        <label className="flex items-center gap-2.5">
          <span className="text-md text-ink">Max pages</span>
          <input
            type="number"
            min={1}
            max={100}
            value={pdf.pdf_max_pages}
            data-testid="pdf-max-pages"
            className="w-16 px-2 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
            onChange={(e) => save({ pdf_max_pages: Math.max(1, Math.min(Number(e.target.value) || 20, 100)) })}
          />
        </label>
        <label className="flex items-center gap-2.5">
          <span className="text-md text-ink">Max size</span>
          <input
            type="number"
            min={1}
            max={10}
            value={pdf.pdf_max_mb}
            data-testid="pdf-max-mb"
            className="w-16 px-2 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
            onChange={(e) => save({ pdf_max_mb: Math.max(1, Math.min(Number(e.target.value) || 10, 10)) })}
          />
          <span className="text-sm text-muted">MB</span>
        </label>
      </div>
      <div className={FIELD_HELP}>
        PDFs over these limits are not attached — you&rsquo;ll see a notice in the composer
        instead.
      </div>
    </div>
  );
}
