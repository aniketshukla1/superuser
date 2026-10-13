import { motion, useReducedMotion } from "framer-motion";
import { cardRise, onScroll, rise, stagger, still } from "../motion";

// Written for two audiences at once: people who would say "local model" unprompted,
// and people who would not. Every card leads with the plain sentence and puts the
// specific name second, so the first audience finds what they were looking for and
// the second never has to look anything up.
//
// The examples deliberately straddle code and everything else — a repo AND an inbox
// — because this is not a coding tool with a chat window bolted on. Six items that
// are true, not eight that are aspirational.
const FEATURES = [
  {
    title: "Any model, including none",
    body: "Run it entirely on your own hardware with a local model, or point it at a hosted one when you want the extra power. Switching is a dropdown, not a reinstall — and the local option needs no key, because there is nobody to bill.",
    icon: <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9zM12 12l8-4.5M12 12v9M12 12L4 7.5" />,
  },
  {
    title: "It uses your actual tools",
    body: "Mail, calendar, chat, repositories, spreadsheets, the browser. Not screenshots of them — the real accounts, doing real things, with your permission asked before anything is sent or changed.",
    icon: <path d="M4 7h16M4 12h16M4 17h10M17 15l3 3 3-3" />,
  },
  {
    title: "Several of them, each different",
    body: "One for your code, one for operations, one for the numbers — each with its own instructions, its own tools and its own model. You pick which one you are talking to; you don't re-explain who you want it to be.",
    icon: <path d="M4 20v-1a5 5 0 0 1 5-5h6a5 5 0 0 1 5 5v1M12 4a4 4 0 1 0 0 8 4 4 0 0 0 0-8z" />,
  },
  {
    title: "It works while you're away",
    body: "Hand it something long and close the laptop. Set a brief that lands before you wake up. Point it at something to watch and it tells you when that thing changes.",
    icon: <path d="M12 8v4l3 2M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16z" />,
  },
  {
    title: "Nothing happens behind your back",
    body: "Every command is shown before it runs, and you decide once whether to approve each one or let it work unattended. Anything it can do, you can watch it do.",
    icon: <path d="M12 5C7 5 3 12 3 12s4 7 9 7 9-7 9-7-4-7-9-7zM12 9.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z" />,
  },
  {
    title: "You can read the memory",
    body: "What it knows about you is plain text files on your disk, not rows in somebody's database. Open them, edit them, delete them. No export request, no waiting period.",
    icon: <path d="M6 4h9l4 4v12H6zM15 4v4h4M9 13h6M9 16h4" />,
  },
];

export function Features() {
  const reduced = useReducedMotion() ?? false;

  return (
    <section>
      <div className="wrap">
        <motion.div
          {...onScroll}
          variants={reduced ? still : rise}
          style={{ display: "flex", flexDirection: "column", gap: "var(--sp-4)" }}
        >
          <p className="eyebrow">The rest of it</p>
          <h2>Everything else, briefly.</h2>
        </motion.div>

        {/* The overshoot easing lives here and only here — cards are the one place
            the motion data says it belongs. Rows and lists get plain deceleration. */}
        <motion.div className="feat-grid" {...onScroll} variants={stagger(0.06)}>
          {FEATURES.map((f) => (
            <motion.article className="feat" key={f.title} variants={reduced ? still : cardRise}>
              <span className="feat-ico" aria-hidden="true">
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  {f.icon}
                </svg>
              </span>
              <h3>{f.title}</h3>
              <p>{f.body}</p>
            </motion.article>
          ))}
        </motion.div>
      </div>
    </section>
  );
}
