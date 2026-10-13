import { Fragment } from "react";
import { motion, useReducedMotion } from "framer-motion";
import { Mark } from "../Mark";
import { charRise, rise, stagger, still } from "../motion";

// Seven words, which is inside the limit the character-stagger preset sets for
// headlines — splitting text makes one element per character, so it is for a line
// and not a paragraph.
//
// The page carries three claims: it stays on your machine, it therefore knows you,
// and the work it returns is therefore right. They are a chain, not a list — so the
// hero states the link the other two hang from. "Can tell everything" is doing two
// jobs at once: it is only safe because nothing leaves, and it is the whole reason
// the thing gets good at your work.
const HEADLINE = "The AI you can tell everything.";

// Split once at module scope: the characters and each word's starting offset into
// the headline, so the per-character delay is computed from the string rather than
// recounted on every render.
const WORDS = (() => {
  let start = 0;
  return HEADLINE.split(" ").map((word) => {
    const entry = { chars: word.split(""), start };
    start += word.length;
    return entry;
  });
})();

export function Hero() {
  const reduced = useReducedMotion() ?? false;

  // Under reduced motion the headline is one plain text node again: no per-character
  // wrappers at all, so there is nothing to animate and nothing extra in the DOM.
  const headline = reduced ? (
    <h1>{HEADLINE}</h1>
  ) : (
    // The whole string stays in the accessibility tree as one label; the visible
    // per-character spans are hidden from it, so a screen reader reads a sentence
    // rather than thirty letters.
    <h1 aria-label={HEADLINE}>
      <motion.span
        aria-hidden="true"
        initial="hidden"
        animate="shown"
        variants={stagger(0.015)}
        style={{ display: "inline" }}
      >
        {/* Words first, characters second — and the spaces BETWEEN the word boxes.
            Two bugs live here, both of which shipped once:
              1. Splitting straight to characters makes every letter an inline-block,
                 so the line may break between any two of them. That rendered
                 "Your coworker run / s on your machine."
              2. Putting the space inside the word's inline-block trims it, because
                 trailing whitespace in an inline-block collapses. That rendered
                 "Yourcoworkerruns onyourmachine."
            So: each word is a nowrap inline-block, and the space is a plain text
            sibling between two of them — breakable, and not inside anything that
            would eat it. */}
        {WORDS.map((word, w) => (
          <Fragment key={`${word.chars.join("")}-${w}`}>
            <span style={{ display: "inline-block", whiteSpace: "nowrap" }}>
              {word.chars.map((ch, c) => (
                <motion.span
                  key={`${ch}-${c}`}
                  className="hero-char"
                  // Index across the whole headline, not per word, so the stagger
                  // reads as one sweep instead of restarting at every space.
                  custom={word.start + c}
                  variants={charRise}
                >
                  {ch}
                </motion.span>
              ))}
            </span>
            {w < WORDS.length - 1 ? " " : null}
          </Fragment>
        ))}
      </motion.span>
    </h1>
  );

  const v = reduced ? still : rise;

  return (
    <section className="hero" id="top">
      <div className="wrap">
        <motion.div
          initial="hidden"
          animate="shown"
          variants={stagger(0.09, reduced ? 0 : 0.45)}
          style={{ display: "flex", flexDirection: "column", gap: "var(--sp-6)" }}
        >
          <motion.p className="eyebrow" variants={v}>
            A coworker that lives on your computer
          </motion.p>

          {headline}

          <motion.p className="lead" variants={v}>
            Assistants forget you between conversations, and the ones that don&rsquo;t are
            keeping what you said on somebody else&rsquo;s servers. Superuser runs on your
            machine — so it can hold your whole working life, and get better at it every
            week, without any of that leaving the room.
          </motion.p>

          <motion.div className="hero-actions" variants={v}>
            <a className="btn btn-primary" href="#start">
              Get started
            </a>
            <a className="btn btn-ghost" href="#tour">
              See it work
            </a>
          </motion.div>

          <motion.p className="hero-note" variants={v}>
            <Mark size={12} /> &nbsp;no account · no telemetry · runs with the Wi-Fi off
          </motion.p>
        </motion.div>
      </div>
    </section>
  );
}
