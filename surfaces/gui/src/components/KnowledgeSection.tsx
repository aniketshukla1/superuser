import { useEffect, useState } from "react";
import { getKnowledge, deleteKnowledgePage, type KnowledgePage } from "../api";
import { Markdown } from "./Markdown";
import { plural } from "../scheduleText";

/** A page's name as a title. Pages written by different runs came back as "status-note",
 *  "Status Brief" and "brief.md" in one list — a slug, a title and a filename. A name that is
 *  plainly a slug or a filename is shown as words; a name someone chose is left as chosen. */
export function pageTitle(raw: string): string {
  const t = raw.trim().replace(/\.md$/i, "");
  if (/\s/.test(t) || /[A-Z]/.test(t)) return t;
  const words = t.replace(/[-_]+/g, " ").trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

const sentence = (s: string) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);

// The knowledge base: what's known about a *topic*.
//
// Sits between the journal (what was learned, and when) and the memory list (what
// arrived, from where). A page here is a subject the assistant chose to write up
// and keeps editing — so unlike a note, the second conversation about Postgres
// improves the Postgres page instead of adding another row.
//
// Pages are editable-by-deletion only: the assistant owns the writing, the user
// owns the right to remove anything wrong.

function groupByCategory(pages: KnowledgePage[]): [string, KnowledgePage[]][] {
  const by = new Map<string, KnowledgePage[]>();
  for (const p of pages) {
    const k = p.category?.trim() || "general";
    by.set(k, [...(by.get(k) ?? []), p]);
  }
  return [...by.entries()].sort((a, b) => b[1].length - a[1].length);
}

export function KnowledgeSection() {
  const [pages, setPages] = useState<KnowledgePage[] | null>(null);
  const [q, setQ] = useState("");
  const [open, setOpen] = useState<KnowledgePage | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmForget, setConfirmForget] = useState(false);
  const pick = (p: KnowledgePage) => {
    setConfirmForget(false);
    setOpen(p);
  };

  const refresh = (query = q) => getKnowledge(query).then(setPages).catch(() => setPages([]));
  useEffect(() => {
    const t = setTimeout(() => refresh(q), 180);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [q]);

  const forget = async (p: KnowledgePage) => {
    setBusy(true);
    const ok = await deleteKnowledgePage(p.slug);
    setBusy(false);
    setConfirmForget(false);
    if (ok) {
      setOpen(null);
      refresh();
    }
  };

  const groups = groupByCategory(pages ?? []);

  // Open on the first page instead of a "Pick a page" pane as wide as the list.
  useEffect(() => {
    if (!open && groups.length) pick(groups[0][1][0]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pages]);

  // Nothing written yet is the common case — say what fills it rather than
  // showing an empty frame.
  if (pages !== null && pages.length === 0 && !q.trim()) {
    return (
      <section className="jrn">
        <div className="jrn-empty">
          Nothing here yet. Topic pages — a concept, a person, a project — appear as subjects come
          up often enough to be worth a reference, and are rewritten in place as they are revisited.
        </div>
      </section>
    );
  }

  return (
    // No heading of its own: the page's lead already says what this section is. The second
    // "Knowledge base" title and its paragraph restated it.
    <section className="jrn mem-tab">
      <div className="mem-head">
        <input
          className="mem-search"
          type="search"
          aria-label="Search pages"
          placeholder="Search pages…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </div>

      {pages === null ? (
        <div className="jrn-empty">Loading…</div>
      ) : pages.length === 0 ? (
        <div className="jrn-empty">Nothing matches “{q}”.</div>
      ) : (
        <div className="mem-split">
          <div className="mem-list">
            <div className="mem-count">
              {pages.length} {pages.length === 1 ? "page" : "pages"}
            </div>
            {groups.map(([cat, items]) => (
              <div key={cat} className="mem-group">
                <div className="mem-group-head">
                  {sentence(cat)} <span className="mem-group-n">{items.length}</span>
                </div>
                {items.map((p) => (
                  <button
                    key={p.slug}
                    className={"mem-row" + (open?.slug === p.slug ? " is-on" : "")}
                    aria-current={open?.slug === p.slug ? "true" : undefined}
                    title={p.title}
                    onClick={() => pick(p)}
                  >
                    <span className="mem-row-title">{pageTitle(p.title)}</span>
                    {p.links.length > 0 && <span className="mem-row-date">{plural(p.links.length, "link")}</span>}
                  </button>
                ))}
              </div>
            ))}
          </div>

          <div className="mem-detail">
            {open ? (
              <>
                <div className="mem-detail-head">
                  <div className="mem-detail-title">{pageTitle(open.title)}</div>
                  {confirmForget ? (
                    <span className="flex items-center gap-2 shrink-0" role="group" aria-label="Confirm forgetting">
                      <button className="btn sm danger" disabled={busy} onClick={() => forget(open)}>
                        Forget it
                      </button>
                      <button className="btn sm ghost" onClick={() => setConfirmForget(false)}>
                        Keep
                      </button>
                    </span>
                  ) : (
                    <button className="mem-forget" disabled={busy} onClick={() => setConfirmForget(true)}>
                      Forget…
                    </button>
                  )}
                </div>
                <div className="mem-meta">{sentence(open.category)}</div>
                <div className="mem-body">
                  <Markdown text={open.body} />
                </div>
                {open.links.length > 0 && (
                  <div className="jrn-actions flex-wrap">
                    {open.links.map((l) => {
                      const target = (pages ?? []).find(
                        (p) => p.title.toLowerCase() === l.toLowerCase(),
                      );
                      return (
                        <button
                          key={l}
                          className="jrn-ghost"
                          disabled={!target}
                          title={target ? `Open ${l}` : `${l} hasn't been written yet`}
                          onClick={() => target && pick(target)}
                        >
                          {l}
                        </button>
                      );
                    })}
                  </div>
                )}
              </>
            ) : (
              <div className="jrn-empty">Pick a page to read it.</div>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
