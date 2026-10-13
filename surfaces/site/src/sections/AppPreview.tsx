import { motion, useReducedMotion } from "framer-motion";
import { onScroll, rise, still } from "../motion";

// The product shot, built as markup rather than a screenshot: it stays sharp at
// every density, it re-themes with the tokens, and it cannot go stale the way a PNG
// of a UI does the first time the real UI changes.
//
// The content is a real exchange from building this app — a Codex turn counting
// Rust files. Invented copy in a product shot is the fastest way to make a landing
// page look like a template.
export function AppPreview() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;

  return (
    <section id="works">
      <div className="wrap">
        <motion.div
          {...onScroll}
          variants={v}
          style={{ display: "flex", flexDirection: "column", gap: "var(--sp-4)" }}
        >
          <p className="eyebrow">Three — so the work comes back right</p>
          <h2>It hands back finished work, not a transcript.</h2>
          <p className="lead">
            Most assistants give you something to then go and do. This one does it: reads
            the files, runs the command, writes the document, and leaves the result on your
            disk in a folder belonging to that conversation.
          </p>
          <p className="lead">
            You see every step as it happens and every command before it runs — so &ldquo;it
            did the work&rdquo; never means &ldquo;you have to trust it did the work&rdquo;.
          </p>
        </motion.div>

        <motion.div
          className="shot"
          {...onScroll}
          variants={v}
          style={{ marginTop: "var(--sp-8)" }}
          aria-label="The Superuser window: sessions on the left, the conversation in the middle, and the live activity panel on the right."
          role="img"
        >
          <div className="shot-bar" aria-hidden="true">
            <span className="shot-dot" />
            <span className="shot-dot" />
            <span className="shot-dot" />
            <span className="shot-title">superuser — codex streaming</span>
          </div>

          <div className="shot-body" aria-hidden="true">
            <div className="shot-side">
              <div className="shot-group">Assistants</div>
              <div className="shot-row on">
                <span className="shot-pip" />
                Assistant
              </div>
              <div className="shot-row">
                <span className="shot-pip" />
                Coder
              </div>
              <div className="shot-row">
                <span className="shot-pip" />
                Ops
              </div>
              <div className="shot-group">Recent</div>
              <div className="shot-row">
                <span className="shot-pip" />
                Codex streaming
              </div>
              <div className="shot-row">
                <span className="shot-pip" />
                Provider endpoints
              </div>
              <div className="shot-row">
                <span className="shot-pip" />
                Weekly plan
              </div>
            </div>

            <div className="shot-main">
              <div className="shot-ask">
                List the .rs files in src-tauri/src/core and say how many.
              </div>
              <div className="shot-tool">
                <span className="shot-live" />
                rg --files src-tauri/src/core -g &apos;*.rs&apos;
                <span className="flag">read-only</span>
              </div>
              <div className="shot-say">
                There are <strong>55</strong> Rust files directly in that directory.
              </div>
              <div className="shot-composer">
                <span className="mono" style={{ color: "var(--accent)" }}>
                  #
                </span>
                Ask the coworker…
              </div>
            </div>

            <div className="shot-rail">
              <div className="shot-mini">ACTIVITY</div>
              <div className="shot-card">
                <span style={{ fontSize: 12 }}>Handed to Codex</span>
                <span className="shot-mini">own tools · read-only sandbox</span>
              </div>
              <div className="shot-card">
                <span style={{ fontSize: 12 }}>1 command</span>
                <span className="shot-mini">approved · 0.4s</span>
              </div>
              <div className="shot-mini" style={{ marginTop: "auto" }}>
                BACKGROUND
              </div>
              <div className="shot-card">
                <span style={{ fontSize: 12 }}>cargo test --lib</span>
                <span className="shot-mini">607 passed</span>
              </div>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  );
}
