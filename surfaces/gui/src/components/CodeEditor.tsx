// Source editing, in a real editor.
//
// This was a `<textarea>`: no line numbers, no syntax highlighting, no bracket
// matching, no undo worth the name. Those are not things to write — CodeMirror is the
// editor half of most code tooling on the web, and building a worse one is the
// mistake this panel already made once with spreadsheets.
//
// The first attempt at this crashed the panel with "Unrecognized extension value in
// extension set": the `@uiw` wrapper resolved `@codemirror/state` 6.7.1 while every
// language grammar resolved 6.7.0, and CodeMirror compares its state classes by
// identity, so each grammar was built against a different `Facet` than the editor
// running it. Fixed in `surfaces/gui/pnpm-workspace.yaml` by pinning one version —
// not here. If this starts throwing that again, look at the dependency tree first.
//
// Both the editor and the grammars load lazily: neither belongs in the bundle of a
// session that only ever opens a spreadsheet.
import { lazy, Suspense, useEffect, useState } from "react";
import type { Extension, ReactCodeMirrorProps } from "@uiw/react-codemirror";
import { themeMode, useTheme } from "../theme";

// `lazy` erases the component's prop types, so they are restored explicitly.
const CodeMirror = lazy(() => import("@uiw/react-codemirror")) as React.ComponentType<ReactCodeMirrorProps>;

/** The grammar for a path, or `null` — an unknown extension is still text, and plain
 *  text in a real editor beats plain text in a textarea. */
async function grammarFor(path: string): Promise<Extension | null> {
  switch ((path.split(".").pop() || "").toLowerCase()) {
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return (await import("@codemirror/lang-javascript")).javascript({ jsx: true });
    case "ts":
    case "tsx":
      return (await import("@codemirror/lang-javascript")).javascript({ jsx: true, typescript: true });
    case "py":
      return (await import("@codemirror/lang-python")).python();
    case "json":
      return (await import("@codemirror/lang-json")).json();
    case "md":
    case "markdown":
      return (await import("@codemirror/lang-markdown")).markdown();
    case "html":
    case "htm":
      return (await import("@codemirror/lang-html")).html();
    case "css":
      return (await import("@codemirror/lang-css")).css();
    case "rs":
      return (await import("@codemirror/lang-rust")).rust();
    default:
      return null;
  }
}

export function CodeEditor({
  value,
  path,
  onChange,
}: {
  value: string;
  path: string;
  onChange: (next: string) => void;
}) {
  // The editor paints immediately and gains highlighting when the grammar arrives.
  // Waiting on a chunk load before showing the file would make opening one feel slow
  // for no gain — and a file with no grammar never waits at all.
  const [extensions, setExtensions] = useState<Extension[]>([]);
  useEffect(() => {
    let live = true;
    setExtensions([]);
    void grammarFor(path).then((g) => {
      if (live && g) setExtensions([g]);
    });
    return () => {
      live = false;
    };
  }, [path]);

  // The editor follows the app's theme (not the Mac's): it should not be the one light
  // rectangle in a dark app, or the reverse.
  const dark = themeMode(useTheme()) === "dark";

  return (
    <Suspense fallback={<div className="rail-muted code-loading">Loading the editor…</div>}>
      <CodeMirror
        className="code-editor"
        value={value}
        height="100%"
        theme={dark ? "dark" : "light"}
        extensions={extensions}
        onChange={onChange}
        basicSetup={{ lineNumbers: true, foldGutter: true, highlightActiveLine: true, autocompletion: false }}
      />
    </Suspense>
  );
}
