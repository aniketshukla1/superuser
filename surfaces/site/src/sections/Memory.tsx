import { motion, useReducedMotion } from "framer-motion";
import { onScroll, rise, stagger, still } from "../motion";

// Link two: because it stays here, it can know you. This is the section the whole
// page exists for — privacy is the precondition, depth is the product.
//
// Shown as a timeline rather than a feature list, because the claim IS the passage
// of time: one conversation is not the thing, ten weeks of them is. The dates are
// relative on purpose; a landing page with fabricated absolute dates ages badly.
const KEPT = [
  {
    when: "week one",
    what: "Learns the shape of your week — what you own, who you answer to, which repo is the one that matters.",
  },
  {
    when: "week three",
    what: "Stops asking. It knows the deploy runs Thursdays and that the Q3 numbers live in a spreadsheet, not the deck.",
  },
  {
    when: "week ten",
    what: "You ask for the usual and get the usual. Nothing to re-explain, no context to paste.",
  },
];

export function Memory() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;

  return (
    <section id="knows">
      <div className="wrap">
        <motion.div
          {...onScroll}
          variants={v}
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "var(--sp-4)",
            maxWidth: "62ch",
          }}
        >
          <p className="eyebrow">Two — so it can know you</p>
          <h2>It has been paying attention since you installed it.</h2>
          <p className="lead">
            Nothing is thrown away when a conversation gets long. What falls out of view is
            written down instead, and once a night the day gets folded into what it already
            knew — so the useful part survives and the noise doesn&rsquo;t.
          </p>
          <p className="lead">
            You can read all of it. It&rsquo;s files on your disk, in plain text, and you
            can delete any of it without asking anyone.
          </p>
        </motion.div>

        <motion.ol className="steps" {...onScroll} variants={stagger(0.07)}>
          {KEPT.map((k) => (
            <motion.li className="step" key={k.when} variants={v}>
              <span className="step-when mono">{k.when}</span>
              <span className="step-what">{k.what}</span>
            </motion.li>
          ))}
        </motion.ol>
      </div>
    </section>
  );
}
