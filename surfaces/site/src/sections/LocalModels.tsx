import { motion, useReducedMotion } from "framer-motion";
import { repoFile } from "../config";
import { SCORE_OF, SCORES } from "../data";
import { EXPO, onScroll, rise, still } from "../motion";

// "Works with local models" is a claim every tool in this space makes and almost none
// measure. This section shows the measurement: the same agent tasks, three runs each,
// out of the same total, so the bars compare like with like. A table underneath in all
// but name — the bars are drawn from the numbers, and the numbers are printed beside them.
export function LocalModels() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;

  return (
    <section id="models" className="band">
      <div className="wrap switch">
        <motion.div {...onScroll} variants={v} className="section-head">
          <p className="eyebrow">Measured, not guessed</p>
          <h2>Good on your own hardware, with no cloud at all.</h2>
          <p className="lead">
            The same 20 agent tasks, three runs each, through Ollama: reading, writing and
            editing files, running commands, recovering from mistakes, admitting what it
            doesn&rsquo;t know, and citing what it read.
          </p>
          <p className="lead">
            For the longest jobs a frontier cloud model is still stronger, and you can give one to
            any assistant that needs it.
          </p>
        </motion.div>

        <motion.figure {...onScroll} variants={v} className="score">
          <figcaption className="score-cap mono">tasks passed, of {SCORE_OF} · cost $0</figcaption>
          {SCORES.map((s, i) => (
            <div key={s.model} className="score-row">
              <span className="score-model mono">{s.model}</span>
              <span className="score-track" aria-hidden="true">
                <motion.span
                  className={`score-fill${i === 0 ? " best" : ""}`}
                  initial={reduced ? false : { width: 0 }}
                  whileInView={{ width: `${(s.passed / SCORE_OF) * 100}%` }}
                  viewport={{ once: true, amount: 0.01 }}
                  transition={reduced ? { duration: 0 } : { duration: 0.9, ease: EXPO, delay: 0.1 * i }}
                  style={reduced ? { width: `${(s.passed / SCORE_OF) * 100}%` } : undefined}
                />
              </span>
              <span className="score-val mono">
                {s.passed}
                <span className="score-of">/{SCORE_OF}</span>
              </span>
            </div>
          ))}
          <a className="score-link" href={repoFile("docs/harness/HARNESS-SCOREBOARD.md")}>
            Every task, every run →
          </a>
        </motion.figure>
      </div>
    </section>
  );
}
