import { motion, useReducedMotion } from "framer-motion";
import { repoFile } from "../config";
import { cardRise, onScroll, rise, stagger, still } from "../motion";

// What the licence lets you do, in plain words, before anyone has to read legal text.
// The two lists carry the meaning in their words, not their colours: "you may" and
// "you may not" read differently before they look different.
const MAY = [
  "Download, build and run it for personal, non-commercial use",
  "Use it at a charity, school, public research body or government institution",
  "Read every line of the source and learn from it",
];
const MAY_NOT = [
  "Use it in or for a business",
  "Change it, or build other work from it",
  "Share copies of it",
];

export function Licence() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;
  const card = reduced ? still : cardRise;

  return (
    <section id="licence">
      <div className="wrap">
        <motion.div {...onScroll} variants={v} className="section-head">
          <p className="eyebrow">The licence</p>
          <h2>Free for personal use.</h2>
          <p className="lead">
            Superuser is source-available under the PolyForm Strict License 1.0.0. Every line is
            on GitHub to read, and it costs nothing to run for yourself.
          </p>
        </motion.div>

        <motion.div className="terms" {...onScroll} variants={stagger(0.08)}>
          <motion.div className="term" variants={card}>
            <h3>You may</h3>
            <ul>
              {MAY.map((m) => (
                <li key={m}>{m}</li>
              ))}
            </ul>
          </motion.div>
          <motion.div className="term term-not" variants={card}>
            <h3>You may not</h3>
            <ul>
              {MAY_NOT.map((m) => (
                <li key={m}>{m}</li>
              ))}
            </ul>
          </motion.div>
        </motion.div>

        <motion.p className="terms-foot" {...onScroll} variants={v}>
          The licence text is what counts: <a href={repoFile("LICENSE")}>read it</a>. Parts that came
          from other projects keep their MIT licences, listed in the{" "}
          <a href={repoFile("THIRD_PARTY_NOTICES.md")}>third-party notices</a>. Need it for your
          business? <a href="#contact">Ask here</a>.
        </motion.p>
      </div>
    </section>
  );
}
