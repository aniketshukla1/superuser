// A fenced code block in the conversation, syntax-highlighted.
//
// Not a CodeMirror instance. The editor is the right tool for a file you are editing
// (see CodeEditor) and the wrong one for a transcript: a turn can carry half a dozen
// fences, and half a dozen editors means half a dozen view trees, keymaps and undo
// histories for text nobody is going to type into. So this uses the same grammars
// through their parser only — `highlightTree` walks the syntax tree and hands back
// ranges with tags on them, which become spans. Static output, no editor.
//
// The grammars load LAZILY and per language, exactly as CodeEditor loads them: a
// conversation with one Python block must not pull in Rust, HTML and CSS to render it.
// Until one arrives the code renders as plain text, so a block is never blank and a
// language nobody ships still reads fine — highlighting is an improvement on plain
// text, never a precondition for showing it.
import { useEffect, useMemo, useRef, useState } from "react";
import { Icon, ICON } from "./Icon";

/** What the fence says, normalised. `js`, `JS` and `javascript` are one language. */
export function normaliseLang(raw?: string): string {
  const l = (raw || "").trim().toLowerCase();
  const alias: Record<string, string> = {
    js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript", node: "javascript",
    ts: "typescript", tsx: "typescript",
    py: "python", python3: "python",
    rs: "rust",
    sh: "shell", bash: "shell", zsh: "shell", console: "shell", shell: "shell",
    yml: "yaml",
    md: "markdown",
    htm: "html",
  };
  return alias[l] ?? l;
}

/** The label shown on the block. Kept separate from the grammar lookup: we say what
 *  the author wrote the fence as, even when nothing highlights it. */
const LABEL: Record<string, string> = {
  javascript: "JavaScript", typescript: "TypeScript", python: "Python", rust: "Rust",
  json: "JSON", html: "HTML", css: "CSS", markdown: "Markdown", shell: "Shell",
  sql: "SQL", yaml: "YAML", toml: "TOML", go: "Go", java: "Java", ruby: "Ruby",
  c: "C", cpp: "C++", diff: "Diff",
};

type Parser = { parse: (input: string) => unknown };

/** The parser for a language, or null when we ship no grammar for it.
 *
 *  Only the grammars already in this app's dependencies — adding a language here
 *  means adding its package, and a highlighter nobody asked for is bundle weight on
 *  every conversation. Everything else falls through to plain text, labelled.
 */
async function parserFor(lang: string): Promise<Parser | null> {
  switch (lang) {
    case "javascript":
      return (await import("@codemirror/lang-javascript")).javascript({ jsx: true }).language.parser;
    case "typescript":
      return (await import("@codemirror/lang-javascript")).javascript({ jsx: true, typescript: true })
        .language.parser;
    case "python":
      return (await import("@codemirror/lang-python")).python().language.parser;
    case "rust":
      return (await import("@codemirror/lang-rust")).rust().language.parser;
    case "json":
      return (await import("@codemirror/lang-json")).json().language.parser;
    case "css":
      return (await import("@codemirror/lang-css")).css().language.parser;
    case "markdown":
      return (await import("@codemirror/lang-markdown")).markdown().language.parser;
    case "html":
      return (await import("@codemirror/lang-html")).html().language.parser;
    default:
      return null;
  }
}

/** Spans for one string, as [text, className] pairs. Plain text is className "". */
export type Piece = [string, string];

/** Split `code` into highlighted pieces using a lezer parser. Exported for the test:
 *  the interesting part is that every character survives, highlighted or not. */
export async function highlight(code: string, parser: Parser): Promise<Piece[]> {
  const { highlightTree, classHighlighter } = await import("@lezer/highlight");
  const tree = parser.parse(code) as Parameters<typeof highlightTree>[0];
  const out: Piece[] = [];
  let at = 0;
  highlightTree(tree, classHighlighter, (from, to, cls) => {
    if (from > at) out.push([code.slice(at, from), ""]);
    out.push([code.slice(from, to), cls]);
    at = to;
  });
  // The tail after the last token, and the whole string when nothing matched.
  if (at < code.length) out.push([code.slice(at), ""]);
  return out;
}

export function CodeBlock({ lang, code }: { lang?: string; code: string }) {
  const language = useMemo(() => normaliseLang(lang), [lang]);
  const [pieces, setPieces] = useState<Piece[] | null>(null);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let stale = false;
    setPieces(null);
    parserFor(language)
      .then((p) => (p ? highlight(code, p) : null))
      .then((out) => {
        if (!stale && out) setPieces(out);
      })
      // A grammar that fails to load or throws leaves plain text on screen, which is
      // the whole content. Highlighting is decoration; losing the code is not an option.
      .catch(() => {});
    return () => {
      stale = true;
    };
  }, [language, code]);

  useEffect(() => () => {
    if (copiedTimer.current) clearTimeout(copiedTimer.current);
  }, []);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      if (copiedTimer.current) clearTimeout(copiedTimer.current);
      copiedTimer.current = setTimeout(() => setCopied(false), 1400);
    } catch {
      /* a denied clipboard is not worth an error card over */
    }
  };

  const label = LABEL[language] || (language ? language : "");

  return (
    <div className="codeblock" data-lang={language || undefined} data-testid="codeblock">
      <div className="codeblock-bar">
        <span className="codeblock-lang">{label}</span>
        <button
          className="codeblock-copy"
          onClick={copy}
          aria-label={copied ? "Copied" : "Copy code"}
          title={copied ? "Copied" : "Copy code"}
          data-testid="codeblock-copy"
        >
          <Icon name={copied ? "check" : "copy"} size={ICON.md} />
        </button>
      </div>
      <pre>
        <code>
          {pieces
            ? pieces.map(([text, cls], i) =>
                cls ? (
                  <span key={i} className={cls}>
                    {text}
                  </span>
                ) : (
                  text
                ),
              )
            : code}
        </code>
      </pre>
    </div>
  );
}
