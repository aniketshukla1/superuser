// Dev-only tracing — present while you are building, absent from what ships.
//
// The webview half of the pair; `src-tauri/src/core/trace.rs` is the other. Two
// independent gates, the same shape the test bridge already uses: `#[cfg(debug_assertions)]`
// in Rust and `import.meta.env.DEV` here. Vite replaces `import.meta.env.DEV` with a
// literal `false` in a production build, so everything below the guard is dead code
// and the minifier removes it — the shipped bundle has no trace output in it.
//
// What survives in production is the call itself, reduced to an empty function. That
// is the honest description and it is why the rule below matters.
//
// # How to call it
//
// Pass a short literal message and put the variable parts in the data object:
//
//     trace("chat", "switching session", { from, to, via: "sidebar" });
//
// NOT a template string. Anything you interpolate is computed at the call site
// whether or not the log is dropped, so a template — or a `.map()`, or a
// `JSON.stringify` — is work a production build still does for a line it will never
// print. An object literal of values you already have is free.
//
// # What may be traced
//
// Shape, not content. Ids, counts, lengths, model names, timings, frame types.
// This is a local-first app whose claim is that conversations stay yours, and a
// trace line is exactly what ends up pasted into a bug report. Use `chars(text)`
// when you need to say something about a message; it reports a length, never words.
//
// # Filtering
//
// Everything is on in a dev build. `localStorage.setItem("trace", "chat,socket")`
// narrows it to those topics; `"off"` silences it. Stored rather than an env var so
// it can be changed in a running app while you are looking at the bug.

/** Guard every call site with this: `TRACE && trace(…)`.
 *
 * Not decoration — it is what makes "omitted" true. Vite replaces
 * `import.meta.env.DEV` with the literal `false` in a production build, so
 * `TRACE && trace(…)` folds to nothing and takes the topic, the message and the data
 * object with it. Without the guard the call survives minification: the function is
 * reduced to `()=>{}`, so nothing PRINTS, but the strings stay in the shipped bundle
 * and the argument objects are still built at runtime for a function that discards
 * them. Measured, not assumed — marking the function pure cleared most call sites and
 * left the ones inside comma sequences behind.
 *
 * So the rule is one line long and mechanical: a trace call always begins `TRACE &&`.
 * `trace.test.ts` checks the call sites, because a rule nobody can forget is better
 * than one everybody must remember. */
export const TRACE = import.meta.env.DEV;

const DEV = TRACE;

/** Topics named in `localStorage.trace`. `null` = all of them. */
function topics(): Set<string> | null {
  try {
    const raw = (localStorage.getItem("trace") || "").trim();
    if (!raw) return null;
    if (raw.toLowerCase() === "off") return new Set();
    return new Set(raw.split(",").map((s) => s.trim().toLowerCase()).filter(Boolean));
  } catch {
    // A private window can throw on access. Tracing must never be the reason
    // something fails, so an unreadable setting reads as "no filter".
    return null;
  }
}

/** Whether a topic prints. Exported for the test; the guard is `enabled` + DEV. */
export function topicEnabled(topic: string, filter: Set<string> | null): boolean {
  return filter === null || filter.has(topic.toLowerCase());
}

/** A length, never the words — see the note above. */
export function chars(text: string | null | undefined): string {
  return `${(text || "").length} chars`;
}

function emit(topic: string, message: string, data?: Record<string, unknown>) {
  if (!topicEnabled(topic, topics())) return;
  // `debug` rather than `log`: browser consoles let you hide this level, so a trace
  // left in place does not drown the warnings that mean something.
  if (data === undefined) console.debug(`[${topic}] ${message}`);
  else console.debug(`[${topic}] ${message}`, data);
}

/** Trace one thing that happened. No-op — and stripped of its body — in production. */
export const trace: (topic: string, message: string, data?: Record<string, unknown>) => void = DEV
  ? emit
  : () => {};
