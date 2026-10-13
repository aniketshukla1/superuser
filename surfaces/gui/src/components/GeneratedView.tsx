/** A view an assistant laid out itself, sealed in a frame that can only draw.
 *
 *  The cards, charts and tables are shapes we drew and the model filled in. This is the
 *  other half: markup the model wrote, for an answer whose shape nothing here anticipated.
 *
 *  What makes that safe is what the frame cannot do, and all of it is enforced here rather
 *  than promised in a prompt:
 *
 *  * `sandbox="allow-scripts"` WITHOUT `allow-same-origin` — the frame is its own origin,
 *    so nothing in it can read this app, its storage, or the API token in it. The two
 *    together would undo the sandbox entirely, which is the one combination to never write.
 *  * No `allow-forms`, `allow-popups`, `allow-modals` or `allow-top-navigation`: a form
 *    inside submits nowhere, a link cannot move the app, and it cannot open a window.
 *  * A Content-Security-Policy of `default-src 'none'` in the document itself: no fetch,
 *    no image from a server, no font, nothing that reaches the network.
 *  * `srcdoc`, so the markup never becomes a URL anything else could load.
 *
 *  The header strip is not decoration. A person scrolling a transcript should never have
 *  to work out whether what they are looking at came from the app or from the model. */
import { useEffect, useMemo, useRef, useState } from "react";
import { Icon, ICON } from "./Icon";

/** Tells the app the document's height whenever it changes. Mirrors
 *  `core::genui::HEIGHT_REPORTER`: the frame is its own origin, so nothing outside can
 *  measure it — but it can say, and this is the one thing it says. */
export const HEIGHT_REPORTER =
  "<script>(function(){var last=0;function send(){" +
  "var h=Math.ceil(document.documentElement.scrollHeight);if(h!==last){last=h;" +
  "parent.postMessage({superuserViewHeight:h},'*');}}addEventListener('load',send);" +
  "if(window.ResizeObserver){new ResizeObserver(send).observe(document.documentElement);}" +
  "setTimeout(send,0);})();</script>";

/** How tall a drawn view may make itself. Below the floor a view is a sliver; above the
 *  ceiling it pushes the conversation off the screen, and anything taller scrolls inside. */
export const VIEW_HEIGHT = { initial: 320, min: 120, max: 720 };

/** The document, policy first. Mirrors `core::genui::document` — the Rust side wraps what
 *  it stores, this wraps what it draws, and both say the same thing. */
export function sealedDocument(html: string): string {
  return (
    `<!doctype html><html><head><meta charset="utf-8">` +
    `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; ` +
    `script-src 'unsafe-inline'; img-src data:; font-src data:; form-action 'none'; base-uri 'none'">` +
    `<style>:root{color-scheme:light dark}html,body{margin:0;padding:0;background:transparent;` +
    `color:var(--ink,#1a1a1a);font:14px/1.5 -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}` +
    `*{box-sizing:border-box;max-width:100%}</style></head><body>${html}${HEIGHT_REPORTER}</body></html>`
  );
}

export function GeneratedView({ title, html, source }: { title: string; html: string; source?: string }) {
  const doc = useMemo(() => sealedDocument(html), [html]);
  const frame = useRef<HTMLIFrameElement>(null);
  const [height, setHeight] = useState(VIEW_HEIGHT.initial);
  // A height from this frame and no other, as a number, clamped. The view's own scripts can
  // send this message too; the worst they can do with it is change their own size.
  useEffect(() => {
    const heard = (e: MessageEvent) => {
      if (!frame.current || e.source !== frame.current.contentWindow) return;
      const h = (e.data as { superuserViewHeight?: unknown } | null)?.superuserViewHeight;
      if (typeof h !== "number" || !Number.isFinite(h)) return;
      setHeight(Math.min(VIEW_HEIGHT.max, Math.max(VIEW_HEIGHT.min, Math.ceil(h))));
    };
    window.addEventListener("message", heard);
    return () => window.removeEventListener("message", heard);
  }, []);
  return (
    <figure className="genview" data-testid="genview">
      <figcaption className="genview-head">
        <Icon name="sparkle" size={ICON.md} className="text-faint shrink-0" />
        <span className="genview-title">{title}</span>
        <span className="genview-note">drawn by the assistant · can’t send anything</span>
      </figcaption>
      <iframe
        ref={frame}
        className="genview-frame"
        style={{ height }}
        title={title}
        data-testid="genview-frame"
        sandbox="allow-scripts"
        referrerPolicy="no-referrer"
        srcDoc={doc}
      />
      {source && <figcaption className="genview-source">Source: {source}</figcaption>}
    </figure>
  );
}
