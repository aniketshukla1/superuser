// A .docx, shown as the document it is.
//
// We cannot render a Word document ourselves — an earlier version listed its
// paragraphs as plain text, which threw away the styles, tables, images and page
// layout that make it a document. `docx-preview` renders it properly, in the browser,
// with no server and no Word install.
//
// There is no editing mode here on purpose. Every other file type in this panel is
// either edited in place (a spreadsheet cell, a source file) or read (a deck, a PDF);
// a document with its own two-mode toggle was the only surface with a third pattern,
// and one pattern per panel is worth more than one extra affordance. The write path
// still exists — `core::docedit` edits paragraphs surgically, and the agent uses it —
// so a document can be changed by asking for it, and the change appears here.
import { useEffect, useRef, useState } from "react";

export function DocEditor({ paragraphs, dataUrl, name }: { paragraphs: string[]; dataUrl?: string; name: string }) {
  const host = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  // A Word page is 8.5in — 816px at 96dpi — wider than this panel, so the document
  // arrives cut off unless it is fitted. Recomputed on resize, but never over a zoom
  // the user set by hand.
  const [zoom, setZoom] = useState(100);
  const chosenByHand = useRef(false);
  const [renderError, setRenderError] = useState("");
  const [loading, setLoading] = useState(true);

  // Lazily imported: a document renderer has no business in the bundle of a session
  // that never opens one.
  useEffect(() => {
    if (!dataUrl) return;
    let dead = false;
    setLoading(true);
    (async () => {
      try {
        const docx = await import("docx-preview");
        if (dead || !host.current) return;
        host.current.innerHTML = "";
        const bytes = Uint8Array.from(atob(dataUrl.split(",")[1] || ""), (c) => c.charCodeAt(0));
        await docx.renderAsync(new Blob([bytes]), host.current, undefined, {
          className: "docxr",
          inWrapper: true,
          breakPages: true,
          ignoreLastRenderedPageBreak: true,
        });
        if (!dead) setLoading(false);
      } catch (e) {
        if (!dead) {
          setRenderError(e instanceof Error ? e.message : String(e));
          setLoading(false);
        }
      }
    })();
    return () => {
      dead = true;
    };
  }, [dataUrl]);

  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const fit = () => {
      if (chosenByHand.current) return;
      const page = host.current?.querySelector("section");
      const pageWidth = page ? page.getBoundingClientRect().width / (zoom / 100) : 816;
      const usable = el.clientWidth - 28;
      if (usable > 100 && pageWidth > 0) {
        setZoom(Math.max(30, Math.min(160, Math.round((usable / pageWidth) * 100))));
      }
    };
    const t = setTimeout(fit, 400);
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => {
      clearTimeout(t);
      ro.disconnect();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dataUrl, loading]);

  const byHand = (next: (z: number) => number) => {
    chosenByHand.current = true;
    setZoom(next);
  };

  return (
    <div className="doc-editor">
      <div className="doc-render" ref={scroller}>
        {renderError && (
          <div className="rail-error doc-note">
            Couldn't render this document: {renderError}. Use “Open in default app” to read it.
          </div>
        )}
        {loading && !renderError && dataUrl && <div className="rail-muted doc-note">Rendering {name}…</div>}
        {!dataUrl && <div className="rail-muted doc-note">This document couldn't be loaded for display.</div>}
        {/* `transform` doesn't change layout size, so the scaled page sits in a box of
            its real on-screen size — otherwise the scroller reserves the full 8.5in
            and shows a scrollbar for space nothing occupies. */}
        <div className="doc-fit" style={{ transform: `scale(${zoom / 100})` }}>
          <div ref={host} />
        </div>
      </div>

      <div className="viewer-foot">
        <span className="rail-muted">
          {paragraphs.length} paragraph{paragraphs.length === 1 ? "" : "s"}
        </span>
        <div className="viewer-zoom">
          <button className="viewer-zoom-btn" onClick={() => byHand((z) => Math.max(30, z - 10))} title="Zoom out">
            −
          </button>
          <span className="rail-muted viewer-zoom-label">{zoom}%</span>
          <button className="viewer-zoom-btn" onClick={() => byHand((z) => Math.min(160, z + 10))} title="Zoom in">
            +
          </button>
        </div>
      </div>
    </div>
  );
}
