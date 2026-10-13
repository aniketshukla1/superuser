import { motion, useReducedMotion } from "framer-motion";
import { CopyBlock } from "../CopyBlock";
import { REPO_URL } from "../config";
import { onScroll, rise, stagger, still } from "../motion";

// The close. It used to end on a "Download for macOS" button with no download behind it.
// There is no signed build yet, so the honest close is the build itself: what you need,
// the commands, and how to make it fully local. Every command here is the README's.
const BUILD = `git clone ${REPO_URL}.git
cd superuser
npm install                     # the Tauri command line
npm --prefix surfaces/gui ci    # the interface
npm run app                     # build the core and open the app`;

const LOCAL = `brew install ollama
brew services start ollama
ollama pull gemma4:12b
# On a Mac with less memory: ollama pull qwen2.5:7b-instruct`;

const NEEDS = ["macOS 13 or later", "Xcode command line tools", "Rust", "Node.js 20+", "CMake"];

export function GetStarted() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;

  return (
    <section className="close" id="start">
      <div className="wrap">
        <motion.div {...onScroll} variants={v} className="section-head center">
          <p className="eyebrow">Get started</p>
          <h2>Start telling it things.</h2>
          <p className="lead">
            Nothing to sign up for and no account to make. It opens against a local model, works
            with the Wi-Fi off, and by next month it will know more about your work than any
            assistant you have to re-introduce yourself to.
          </p>
        </motion.div>

        <motion.ul className="needs" {...onScroll} variants={stagger(0.04)} aria-label="What you need">
          {NEEDS.map((n) => (
            <motion.li key={n} variants={v} className="chip">
              {n}
            </motion.li>
          ))}
        </motion.ul>

        <motion.div className="cmds" {...onScroll} variants={v}>
          <CopyBlock label="Build and run" code={BUILD} />
          <CopyBlock label="Optional: fully local, no keys" code={LOCAL} />
        </motion.div>

        <motion.p className="hero-note start-note" {...onScroll} variants={v}>
          The first build compiles the Rust core, so it takes several minutes. On first launch,
          setup asks which model to use; choose Ollama to stay on this Mac.
        </motion.p>

        <motion.div className="hero-actions center" {...onScroll} variants={v}>
          <a className="btn btn-primary" href={REPO_URL}>
            Open on GitHub
          </a>
          <a className="btn btn-ghost" href={`${REPO_URL}#readme`}>
            Read the full guide
          </a>
        </motion.div>
      </div>
    </section>
  );
}
