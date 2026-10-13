import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { useEffect, useRef, useState } from "react";
import { TOUR } from "../data";
import { onScroll, rise, still } from "../motion";

// The product, shown as itself: real screens from the app, running on invented demo data
// (surfaces/gui/e2e-demo). The markup mock in AppPreview shows one exchange in detail;
// this shows the breadth, one screen at a time, with the sentence that says why it matters.
//
// It advances on its own only while it is on screen and nobody is pointing at it, and
// stops for good once someone picks a screen: a carousel that moves on while you are
// reading it is the thing people hate about carousels. Under reduced motion it starts
// paused and swaps without the fade.

const HOLD_MS = 5200;
const src = (file: string) => `${import.meta.env.BASE_URL}tour/${file}.webp`;

export function Tour() {
  const reduced = useReducedMotion() ?? false;
  const [index, setIndex] = useState(0);
  const [auto, setAuto] = useState(!reduced);
  const [hover, setHover] = useState(false);
  const [inView, setInView] = useState(false);
  const stage = useRef<HTMLDivElement>(null);
  const playing = auto && !hover && inView;

  useEffect(() => {
    const el = stage.current;
    if (!el) return;
    const io = new IntersectionObserver(([entry]) => setInView(entry.isIntersecting), { threshold: 0.35 });
    io.observe(el);
    return () => io.disconnect();
  }, []);

  useEffect(() => {
    if (!playing) return;
    const t = window.setTimeout(() => setIndex((n) => (n + 1) % TOUR.length), HOLD_MS);
    return () => window.clearTimeout(t);
  }, [index, playing]);

  // The next frame is fetched while this one is on show, so the change never flashes.
  useEffect(() => {
    const next = new Image();
    next.src = src(TOUR[(index + 1) % TOUR.length].file);
  }, [index]);

  const frame = TOUR[index];
  const fade = reduced ? { duration: 0 } : { duration: 0.45, ease: "easeOut" as const };

  return (
    <section id="tour" className="tour-section" aria-label="A tour of Superuser">
      <div className="wrap">
        <motion.div
          className="shot tour"
          {...onScroll}
          variants={reduced ? still : rise}
          onMouseEnter={() => setHover(true)}
          onMouseLeave={() => setHover(false)}
        >
          <div className="shot-bar" aria-hidden="true">
            <span className="shot-dot" />
            <span className="shot-dot" />
            <span className="shot-dot" />
            <span className="shot-title">superuser — {frame.window}</span>
          </div>
          <div className="tour-stage" ref={stage}>
            <AnimatePresence initial={false}>
              <motion.img
                key={frame.file}
                src={src(frame.file)}
                alt={frame.alt}
                width={1600}
                height={1000}
                decoding="async"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={fade}
              />
            </AnimatePresence>
          </div>
        </motion.div>

        <p className="tour-caption" aria-live="polite">
          {frame.caption}
        </p>

        <div className="tour-steps" role="group" aria-label="Screens in the tour">
          {TOUR.map((t, n) => (
            <button
              key={t.file}
              type="button"
              className="tour-step"
              aria-pressed={n === index}
              onClick={() => {
                setIndex(n);
                setAuto(false);
              }}
            >
              {t.label}
              {n === index && playing && (
                <span key={`run-${index}`} className="tour-run" style={{ animationDuration: `${HOLD_MS}ms` }} />
              )}
            </button>
          ))}
          <button
            type="button"
            className="tour-step tour-toggle"
            onClick={() => setAuto((a) => !a)}
            aria-label={auto ? "Pause the tour" : "Play the tour"}
          >
            {auto ? "Pause" : "Play"}
          </button>
        </div>
      </div>
    </section>
  );
}
