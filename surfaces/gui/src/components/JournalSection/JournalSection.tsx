import { useEffect, useState } from "react";
import { getJournal, runDream, setCoreMemory, undoEvolution, type Journal, type JournalEntry } from "../../api";
import { Markdown } from "../Markdown";
import { TAB_HINT, journalOf, type Tab } from "./journalOf";
import { PatternsView } from "./PatternsView";

export function JournalSection() {
  const [j, setJ] = useState<Journal | null>(null);
  const [tab, setTab] = useState<Tab>("core");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [draft, setDraft] = useState<string | null>(null);

  const load = () => {
    getJournal()
      .then((raw) => setJ(journalOf(raw)))
      .catch(() => setJ(null));
  };
  useEffect(load, []);

  const consolidate = async () => {
    setBusy(true);
    setNotice(null);
    const r = await runDream(3);
    setNotice(
      r.status === "consolidated"
        ? `Consolidated into ${r.entries} core ${r.entries === 1 ? "entry" : "entries"}.`
        : r.status === "skipped"
          ? `Nothing to consolidate — ${r.reason}.`
          : `Couldn't consolidate: ${r.reason}`,
    );
    setBusy(false);
    load();
  };

  const save = async () => {
    if (draft === null) return;
    setBusy(true);
    await setCoreMemory(draft);
    setBusy(false);
    setDraft(null);
    load();
  };

  const undo = async () => {
    setBusy(true);
    setNotice(null);
    const r = await undoEvolution();
    setNotice(
      r.ok
        ? `Put back ${r.restored} file${r.restored === 1 ? "" : "s"} from before the last pass.`
        : `Couldn't undo: ${r.error ?? "nothing to undo"}`,
    );
    setBusy(false);
    load();
  };

  if (!j) return null;

  const recaps = j.recaps ?? [];
  const patterns = j.wiki?.patterns ?? [];
  // Only the rejections are worth showing. An accepted change is already visible as
  // the skill it produced; a rejected one leaves no other trace, and it is the row
  // that stops the same edit being proposed a third time.
  const rejected = (j.wiki?.impacts ?? []).filter((i) => i.verdict === "undone").reverse();
  const entries: JournalEntry[] =
    tab === "daily"
      ? j.dailies
      : tab === "recaps"
        ? recaps
        : tab === "dreams"
          ? j.dreams
          : j.evolutions;

  return (
    <section className="jrn">
      {/* No heading of its own: the page already names this section and says what it is
          (BrainView's lead), and a second title with a second paragraph restating it pushed
          the content below the fold. The views and the one action share a toolbar. */}
      <div className="jrn-toolbar">
        <div className="jrn-tabs" role="group" aria-label="Timeline views">
          {(
            [
              ["core", "Core memory", j.core_entries],
              ["daily", "Daily", j.dailies.length],
              ["recaps", "Recaps", recaps.length],
              ["dreams", "Dreams", j.dreams.length],
              ["learned", "Learned", j.evolutions.length],
              ["patterns", "Patterns", patterns.length],
            ] as [Tab, string, number][]
          ).map(([key, label, n]) => (
            <button
              key={key}
              className={"jrn-tab" + (tab === key ? " is-on" : "") + (n === 0 ? " is-empty" : "")}
              aria-pressed={tab === key}
              title={TAB_HINT[key]}
              onClick={() => setTab(key)}
            >
              {label} <span className="jrn-n">{n}</span>
            </button>
          ))}
        </div>
        <button className="btn sm shrink-0" disabled={busy} onClick={consolidate}>
          {busy ? "Working…" : "Consolidate now"}
        </button>
      </div>
      {!j.nightly_enabled && tab !== "learned" && tab !== "patterns" && (
        <div className="jrn-off">Nightly consolidation is off.</div>
      )}
      {!j.evolution_enabled && (tab === "learned" || tab === "patterns") && (
        <div className="jrn-off">Self-learning is off.</div>
      )}

      {notice && <div className="jrn-notice">{notice}</div>}

      {tab === "core" ? (
        draft !== null ? (
          <>
            <textarea
              className="jrn-editor"
              aria-label="Core memory"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
            <div className="jrn-actions">
              <button className="btn-primary sm" disabled={busy} onClick={save}>Save</button>
              <button className="btn sm ghost" onClick={() => setDraft(null)}>Cancel</button>
            </div>
          </>
        ) : (
          <>
            {/* Rendered: the core is markdown, and it was shown as source in a code box. */}
            {j.core.trim() ? (
              <div className="jrn-body">
                <Markdown text={j.core} />
              </div>
            ) : (
              <div className="jrn-empty">
                Nothing distilled yet. Core memory fills in once conversations run long enough to be
                summarized, then the nightly pass condenses them.
              </div>
            )}
            <div className="jrn-actions">
              <button className="btn sm" onClick={() => setDraft(j.core)}>
                {j.core.trim() ? "Edit" : "Write it yourself"}
              </button>
            </div>
          </>
        )
      ) : tab === "patterns" ? (
        <PatternsView patterns={patterns} rejected={rejected} />
      ) : entries.length === 0 ? (
        <div className="jrn-empty">
          {tab === "daily"
            ? "No daily memory yet. It's written when a conversation grows past its context window — the turns that would otherwise be forgotten, kept as they were said."
            : tab === "recaps"
              ? "No recaps yet. Each time turns fall out of a conversation they're summarized, and the summary is filed here beside the words it came from."
            : tab === "dreams"
              ? "No dreams yet. Each consolidation leaves a note here saying what it merged, learned or corrected."
              : "Nothing learned yet. After a conversation goes quiet it gets reviewed, and if that changes a skill or a remembered fact, the change is recorded here. Most conversations change nothing."}
        </div>
      ) : (
        <>
          {tab === "learned" && (
            <div className="jrn-actions mt-0 mb-2.5">
              <button className="btn sm" disabled={busy || !j.can_undo} onClick={undo}>
                {j.can_undo ? "Undo the last pass" : "Nothing to undo"}
              </button>
            </div>
          )}
          <div className="jrn-entries">
            {entries.map((e) => (
              <div key={e.date} className="jrn-entry">
                <div className="jrn-date">{e.date}</div>
                <div className="jrn-body">
                  <Markdown text={e.body.trim()} />
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </section>
  );
}
