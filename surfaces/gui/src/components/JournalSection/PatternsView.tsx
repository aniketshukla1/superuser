import type { Journal } from "../../api";
import { Markdown } from "../Markdown";

type Wiki = NonNullable<Journal["wiki"]>;

/** The Patterns tab: what reviews have learned, and the changes you turned down. */
export function PatternsView({ patterns, rejected }: { patterns: Wiki["patterns"]; rejected: Wiki["impacts"] }) {
  return (
    patterns.length === 0 && rejected.length === 0 ? (
      <div className="jrn-empty">
        Nothing noticed yet. When a conversation goes quiet it gets reviewed, and
        anything the review learns about how the work goes — a wall it hit, an
        approach that worked — is written here. These outlive an undo: putting a
        skill back keeps the reason it was changed.
      </div>
    ) : (
      <>
        <div className="jrn-entries">
          {patterns.map((pt) => (
            <div key={pt.id} className="jrn-entry">
              <div className="wiki-head">
                <span className={"wiki-kind is-" + pt.kind}>{pt.kind}</span>
                <span className="wiki-title">{pt.title}</span>
                {/* Recurrence, not recency, is what earns a skill change — so it
                    is called out rather than buried in the body. */}
                <span className="wiki-seen" title={`First noticed ${pt.first_seen || "—"}`}>
                  seen {pt.seen}×
                </span>
              </div>
              <div className="jrn-body">
                <Markdown text={pt.body.trim()} />
              </div>
            </div>
          ))}
        </div>
        {rejected.length > 0 && (
          <div className="wiki-rejected">
            <h4 className="wiki-rejected-h">Changes you turned down</h4>
            <p className="jrn-sub">
              Kept on purpose. Every later review is shown these, so a change you
              undid does not come back.
            </p>
            {rejected.map((im, n) => (
              <div key={im.snapshot + n} className="wiki-row">
                <code className="wiki-target">{im.target}</code>
                <span className="wiki-note">{im.note || "(no note)"}</span>
                <span className="wiki-at">{im.at}</span>
              </div>
            ))}
          </div>
        )}
      </>
    )
  );
}
