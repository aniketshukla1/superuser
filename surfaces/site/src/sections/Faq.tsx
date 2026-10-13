import { motion, useReducedMotion } from "framer-motion";
import { onScroll, rise, stagger, still } from "../motion";

// The questions people actually ask, answered the way the README answers them. Native
// <details>, so every answer opens without script and is found by the browser's search.
const FAQ: { q: string; a: string }[] = [
  {
    q: "Does my data leave my computer?",
    a: "Only when you choose a cloud model or connect an app, and then only to that provider or app. Each request is listed on the Network page with its host and what it was for. With a local model and local-only mode on, nothing leaves, and the refusals show up in the same list.",
  },
  {
    q: "Do I need an account or an API key?",
    a: "No account. With Ollama you need no key at all, and a cloud model needs only that provider's key, stored encrypted on your Mac. Superuser Cloud, coming next, will be an optional account for one-click connections; everything works without it.",
  },
  {
    q: "Is it safe to let it act?",
    a: "By default, anything that sends, runs or changes something asks first. Boundaries refuse whole kinds of action outright, and every decision lands in an audit trail where each entry is chained to the last, so an edit shows. You choose which assistants may work unattended, and what they may do when they do.",
  },
  {
    q: "How is it different from ChatGPT or Claude's own apps?",
    a: "Those run the assistant in the provider's cloud and keep its memory there. Superuser runs the assistant on your Mac, keeps its memory as files you own, works inside your apps with approvals and an audit trail, and can run with no cloud at all.",
  },
  {
    q: "Where does my data live?",
    a: "In ~/Library/Application Support/ai.superuser.app: the memory (markdown files), conversations, the audit trail and your encrypted keys. Delete the folder and it's gone.",
  },
  {
    q: "Is it open source?",
    a: "It's source-available: every line is on GitHub, and you may build and run it for personal use under the PolyForm Strict License. It doesn't allow business use, changes or redistribution. Parts that came from MIT-licensed projects keep their MIT licences.",
  },
  {
    q: "Can I use it at work?",
    a: "Not under the free licence. For a business licence, use the contact form above.",
  },
  {
    q: "Windows or Linux?",
    a: "Not yet. Superuser is built and tested on macOS 13 and later.",
  },
];

export function Faq() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;

  return (
    <section id="faq">
      <div className="wrap faq-wrap">
        <motion.div {...onScroll} variants={v} className="section-head">
          <p className="eyebrow">Questions</p>
          <h2>Asked before you could.</h2>
        </motion.div>
        <motion.div className="faq" {...onScroll} variants={stagger(0.03)}>
          {FAQ.map((f) => (
            <motion.details key={f.q} variants={v}>
              <summary>{f.q}</summary>
              <p>{f.a}</p>
            </motion.details>
          ))}
        </motion.div>
      </div>
    </section>
  );
}
