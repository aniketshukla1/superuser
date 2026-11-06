import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import remarkGfm from "remark-gfm";
import { CodeBlock } from "./CodeBlock";
import { Icon, ICON } from "./Icon";

// §34 (UX-016): the agent ends a deliverable turn with plain markdown —
// [Title](artifact:relative/path) — and the renderer turns it into a chip that opens the
// artifact viewer in place. Plumbing is a window event (the viewer lives in WorkspacePanel;
// this component renders deep inside the transcript): WorkspacePanel resolves the path against
// the session's artifact list, App un-hides the rail.
export const OPEN_ARTIFACT_EVENT = "superuser-open-artifact";

function ArtifactChip({ path, title }: { path: string; title: string }) {
  const file = path.split("/").pop() || path;
  return (
    <button
      className="art-chip"
      data-testid="artifact-chip"
      title={path}
      onClick={() =>
        window.dispatchEvent(new CustomEvent(OPEN_ARTIFACT_EVENT, { detail: { path } }))
      }
    >
      <span className="art-chip-ico">
        <Icon name="file" size={ICON.md} />
      </span>
      <span className="art-chip-meta">
        <b>{title || file}</b>
        {title && title !== file && <span>{file}</span>}
      </span>
      <span className="art-chip-open">Open ›</span>
    </button>
  );
}

// Assistant messages rendered as GitHub-flavored markdown (headings, lists, tables, code,
// links). Links open externally — never navigate the app shell — except artifact: links,
// which open the session's artifact viewer.
export function Markdown({ text }: { text: string }) {
  return (
    <div className="md">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        // artifact: is ours — keep it through the sanitizer (everything else gets the default
        // http/https/mailto policy).
        urlTransform={(url) => (url.startsWith("artifact:") ? url : defaultUrlTransform(url))}
        components={{
          // A fenced block becomes a real code block: language label, copy button,
          // syntax highlighting. `inline` code (`like this`) stays a plain <code> —
          // react-markdown routes both through this one component, and the fence is
          // the one carrying a `language-x` class.
          code: ({ node: _n, className, children, ...props }) => {
            const lang = /language-([\w+-]+)/.exec(className || "")?.[1];
            const text = String(children ?? "");
            // No language and no newline is a word in a sentence, not a program.
            if (!lang && !text.includes("\n")) {
              return (
                <code className={className} {...props}>
                  {children}
                </code>
              );
            }
            return <CodeBlock lang={lang} code={text.replace(/\n$/, "")} />;
          },
          // react-markdown wraps a fence in <pre>; CodeBlock brings its own, so this
          // would otherwise nest one inside the other and double the padding.
          pre: ({ children }) => <>{children}</>,
          a: ({ node: _n, href, children, ...props }) => {
            if (href?.startsWith("artifact:")) {
              const title = Array.isArray(children) ? children.join("") : String(children ?? "");
              return <ArtifactChip path={href.slice("artifact:".length)} title={title} />;
            }
            return (
              <a href={href} {...props} target="_blank" rel="noreferrer">
                {children}
              </a>
            );
          },
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
