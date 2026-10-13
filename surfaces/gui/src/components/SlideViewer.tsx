// A .pptx, drawn as the deck it is.
//
// The first attempt read the shape geometry out of the file and drew it here. That
// was the wrong shape of solution, and measuring a real deck showed why: it had
// **zero** `<a:off>` elements across ten slides — every position, size, colour and
// font inherited from its layout, master and theme. Drawing it meant resolving all
// three, which is reimplementing PowerPoint, and it would still have been wrong on
// charts, tables, SmartArt and gradients.
//
// `pptx-preview` already does that resolution and renders to **SVG**, so a slide is
// vector at any zoom rather than a bitmap that softens. It is MIT-licensed, runs
// entirely in the browser, and needs no server and no Office install — which is the
// only kind of answer this app can ship.
//
// Loaded lazily: a deck renderer has no business in the bundle of a session that
// never opens one.
import { useEffect, useRef, useState } from "react";

/** Rendered slide width in CSS pixels at 100%. The library lays out to a fixed
 *  width and derives the height from the deck's own aspect ratio. */
const BASE_WIDTH = 960;

export function SlideViewer({ dataUrl, name }: { dataUrl: string; name: string }) {
  const host = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  // Fit-to-width, not a fixed percentage: opening a deck should show a slide, and
  // this panel is resizable, so any constant is wrong at some width. Recomputed
  // whenever the panel changes size, unless the user has chosen a zoom themselves —
  // overriding a deliberate choice on every drag would be the Browser-tab mistake.
  const [zoom, setZoom] = useState(60);
  const chosenByHand = useRef(false);
  const [error, setError] = useState("");
  const [slides, setSlides] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let dead = false;
    setError("");
    setLoading(true);
    setSlides(0);
    (async () => {
      try {
        const { init } = await import("pptx-preview");
        if (dead || !host.current) return;
        host.current.innerHTML = "";
        const bytes = Uint8Array.from(atob(dataUrl.split(",")[1] || ""), (c) => c.charCodeAt(0));
        const previewer = init(host.current, { width: BASE_WIDTH, height: Math.round(BASE_WIDTH * 0.5625) });
        await previewer.preview(bytes.buffer);
        if (dead) return;
        // Count what actually landed, so "8 slides" is the deck rather than a guess.
        setSlides(host.current.querySelectorAll('[class*="slide-wrapper-"]').length);
        setLoading(false);
      } catch (e) {
        if (!dead) {
          setError(e instanceof Error ? e.message : String(e));
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
      const usable = el.clientWidth - 32; // the scroller's own padding
      if (usable > 100) setZoom(Math.max(20, Math.min(200, Math.round((usable / BASE_WIDTH) * 100))));
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [slides]);

  const setZoomByHand = (next: (z: number) => number) => {
    chosenByHand.current = true;
    setZoom(next);
  };

  return (
    <div className="slides-viewer">
      {error && (
        <div className="rail-error slides-note">
          Couldn't render this deck: {error}. Use “Open in default app” to see it.
        </div>
      )}
      {loading && !error && <div className="rail-muted slides-note">Rendering {name}…</div>}
      <div className="slides-scroll" ref={scroller}>
        {/* The library sizes its own output; the zoom is applied to the whole
            rendering rather than re-laying it out, so a slide never reflows into a
            layout PowerPoint would not show. It stays sharp because the output is
            SVG. */}
        {/* `transform` doesn't change layout size, so the scaled rendering is wrapped
            in a box of its real on-screen size — without it the scroller reserves the
            unscaled width and shows a horizontal scrollbar for space nothing uses. */}
        <div className="slides-fit" style={{ width: BASE_WIDTH * (zoom / 100) }}>
          <div
            className="slides-stage"
            style={{ transform: `scale(${zoom / 100})`, width: BASE_WIDTH }}
            ref={host}
          />
        </div>
      </div>
      <div className="viewer-foot">
        <span className="rail-muted">
          {slides > 0 ? `${slides} slide${slides === 1 ? "" : "s"}` : " "}
        </span>
        <div className="viewer-zoom">
          <button className="viewer-zoom-btn" onClick={() => setZoomByHand((z) => Math.max(20, z - 10))} title="Zoom out">
            −
          </button>
          <span className="rail-muted viewer-zoom-label">{zoom}%</span>
          <button className="viewer-zoom-btn" onClick={() => setZoomByHand((z) => Math.min(200, z + 10))} title="Zoom in">
            +
          </button>
        </div>
      </div>
    </div>
  );
}
