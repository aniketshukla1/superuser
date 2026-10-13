import { motion, useReducedMotion } from "framer-motion";
import { onScroll, rise, stagger, still } from "../motion";

// Link one of the chain: it stays here. Everything the page claims afterwards
// depends on this being true, so it comes first and it shows the receipt.
//
// Every product in this category says "private". The difference worth showing is
// that the app keeps a log of what left, with the host and the reason — so the
// claim is checkable rather than promised. An illustration of a padlock would be
// the version of this section that means nothing.
const LINES = [
  { verb: "POST", host: "127.0.0.1:11434", why: "local model", blocked: false },
  { verb: "GET", host: "127.0.0.1:8765", why: "this app", blocked: false },
  { verb: "POST", host: "api.openai.com", why: "refused", blocked: true },
  { verb: "GET", host: "calendar.google.com", why: "refused", blocked: true },
];

export function LocalOnly() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;

  return (
    <section id="stays">
      <div className="wrap switch">
        <motion.div
          {...onScroll}
          variants={v}
          style={{ display: "flex", flexDirection: "column", gap: "var(--sp-4)" }}
        >
          <p className="eyebrow">One — it stays here</p>
          <h2>Private is a setting you can check.</h2>
          <p className="lead">
            The model can run on your own hardware, so an ordinary conversation involves no
            network at all. Turn on local-only and every outbound request is refused —
            enforced at the single point they all pass through, so it can&rsquo;t be true in
            one corner of the app and quietly false in another.
          </p>
          <p className="lead">
            Leave it off and the log keeps going anyway: what left, where it went, and which
            feature asked for it. If the setting can&rsquo;t be read, the answer is no.
          </p>
        </motion.div>

        <motion.div
          className="log"
          {...onScroll}
          variants={stagger(0.05)}
          role="img"
          aria-label="A log of outbound requests: two to this machine were allowed, and two to the internet were refused because local-only mode is on."
        >
          {LINES.map((l) => (
            <motion.div
              className={`log-row${l.blocked ? " blocked" : ""}`}
              key={l.host}
              variants={v}
            >
              <span className="verb">{l.verb}</span>
              <span className="host">{l.host}</span>
              {/* The word carries the meaning, not the colour — the two states read
                  as different text before they read as different hues. */}
              <span className="why">{l.why}</span>
            </motion.div>
          ))}
        </motion.div>
      </div>
    </section>
  );
}
