import { useEffect, useState } from "react";
import { listMemories, getMemory, deleteMemory, type MemoryRow } from "../../api";
import { Markdown } from "../Markdown";
import { exactTime, shortDate } from "../../when";
import { groupByCategory, readableStamps, titleParts } from "./groupByCategory";
import { LedgerStrip } from "./LedgerStrip";

export function MemoriesTab() {
  const [rows, setRows] = useState<MemoryRow[] | null>(null);
  const [q, setQ] = useState("");
  const [open, setOpen] = useState<MemoryRow | null>(null);
  const [busy, setBusy] = useState(false);
  // Forgetting deletes the file; it asks once, the way deleting anything else here does.
  const [confirmForget, setConfirmForget] = useState(false);

  const refresh = (query = q) => {
    listMemories(query)
      .then(setRows)
      .catch(() => setRows([]));
  };

  // Debounced so typing doesn't fire a request per keystroke.
  useEffect(() => {
    const t = setTimeout(() => refresh(q), 180);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [q]);

  const pick = async (m: MemoryRow) => {
    setConfirmForget(false);
    setOpen(m);
    const full = await getMemory(m.filename);
    if (full) setOpen(full);
  };

  const forget = async (m: MemoryRow) => {
    setBusy(true);
    const ok = await deleteMemory(m.filename);
    setBusy(false);
    setConfirmForget(false);
    if (ok) {
      setOpen(null);
      refresh();
    }
  };

  const groups = groupByCategory(rows ?? []);

  // The reader opens on the first memory rather than on "Pick a memory" beside the list: an
  // empty pane half the width of the page was the first thing the tab showed.
  useEffect(() => {
    if (!open && groups.length) pick(groups[0][1][0]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows]);

  // Nothing to search in an empty vault. Kept while loading (so a full vault doesn't jump) and
  // while a query is typed (so a search that matches nothing can be changed).
  const searchable = rows === null || rows.length > 0 || q !== "";

  return (
    <section className="mem-tab">
      {/* The heading is the page's (BrainView's PanelHead); this row carries only search. */}
      {searchable && (
        <div className="mem-head">
          <input
            className="mem-search"
            type="search"
            aria-label="Search memories"
            placeholder="Search memories…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>
      )}

      <LedgerStrip />

      {rows === null ? (
        <div className="mem-empty">Loading…</div>
      ) : rows.length === 0 ? (
        q ? (
          <div className="mem-empty">Nothing matches “{q}”.</div>
        ) : (
          <div className="mem-empty">
            No memories yet. They arrive from connected apps and from finished conversations.{" "}
            <a className="text-accent hover:underline whitespace-nowrap" href="#/connectors">
              Connect an app ›
            </a>
          </div>
        )
      ) : (
        <div className="mem-split">
          <div className="mem-list">
            <div className="mem-count">
              {rows.length} {rows.length === 1 ? "memory" : "memories"}
            </div>
            {groups.map(([cat, items]) => (
              <div key={cat} className="mem-group">
                <div className="mem-group-head">
                  {cat} <span className="mem-group-n">{items.length}</span>
                </div>
                {items.map((m) => {
                  const { kind, name } = titleParts(m.title || m.filename);
                  return (
                    <button
                      key={m.filename}
                      className={"mem-row" + (open?.filename === m.filename ? " is-on" : "")}
                      aria-current={open?.filename === m.filename ? "true" : undefined}
                      title={m.title || m.filename}
                      onClick={() => pick(m)}
                    >
                      <span className="mem-row-title">
                        {name}
                        {kind && <span className="mem-row-kind">{kind}</span>}
                      </span>
                      <span className="mem-row-date" title={exactTime(m.date)}>
                        {shortDate(m.date)}
                      </span>
                    </button>
                  );
                })}
              </div>
            ))}
          </div>

          <div className="mem-detail">
            {open ? (
              <>
                <div className="mem-detail-head">
                  <div className="mem-detail-title">{titleParts(open.title || open.filename).name}</div>
                  {confirmForget ? (
                    <span className="flex items-center gap-2 shrink-0" role="group" aria-label="Confirm forgetting">
                      <button className="btn sm danger" disabled={busy} onClick={() => forget(open)} data-testid="mem-forget-confirm">
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
                <div className="mem-meta">
                  {[titleParts(open.title || open.filename).kind, open.category, shortDate(open.date), ...(open.tags ?? [])]
                    .filter(Boolean)
                    .join(" · ")}
                </div>
                {/* Where the words came from. It changes what the assistant is
                    allowed to do after reading them, so it belongs on the page
                    rather than only in the file. */}
                {open.origin !== "internal" ? (
                  <div className="mem-origin" data-testid="mem-origin-external">
                    From outside — synced from an app, a server or the web. Nobody here wrote it,
                    so an automation that reads it will ask before sending or running anything.
                  </div>
                ) : (
                  <div className="mem-origin is-internal" data-testid="mem-origin-internal">
                    Written here — by you, or by the app about itself.
                  </div>
                )}
                {open.summary && <p className="mem-summary">{open.summary}</p>}
                {/* Rendered, not printed. The body is markdown and was shown as source in a code
                    box — "**aniketshukla1/mem0**" with the asterisks, "- Last push:
                    2026-07-30T14:11:10Z" with the raw stamp. */}
                <div className="mem-body">
                  {open.body === undefined ? "…" : <Markdown text={readableStamps(open.body)} />}
                </div>
                <div className="mem-file">{open.filename}</div>
              </>
            ) : (
              <div className="mem-empty">Pick a memory to read it.</div>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
