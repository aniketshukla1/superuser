/** A summary the agent composed: a title, the facts that matter, and what it means.
 *
 * The shape a result usually wants when it is neither a chart nor a table — a runway
 * date, a variance total, a scan's headline. Prose can say all of it, and does say it
 * worse: the numbers end up in a sentence where they cannot be scanned, compared, or
 * found again three turns later.
 *
 * Deliberately READ-ONLY. No buttons, no inputs, nothing that makes anything happen.
 * This app already has considered ways for the agent to act — `ask_user` for a
 * decision, `propose_plan` for a plan, the approval card for a command — and a second
 * one hiding inside a summary block would be a new authority nobody reviewed. It is
 * also why there is no free-form layout here: a card is four fixed slots, so text the
 * agent scraped off a web page can fill them and can do nothing else.
 */
import { Icon, ICON } from "./Icon";

export type Fact = {
  label: string;
  value: string;
  /** The one number the card is about, if there is one. */
  emphasis?: boolean;
  /** Checklist only: whether that one passed. An entry whose state is unsaid is the one
   *  a person assumes passed, so the agent is made to say it. */
  done?: boolean;
  /** Metrics only: how the figure moved — "+12% on last week". */
  delta?: string;
};

export type CardSpec = {
  title?: string;
  /** What it means, in the agent's words. A card of figures nobody reads is a table. */
  note?: string;
  /** Colours the left edge only. Never the text — a red number is unreadable at 12px
   *  on this ground, and the tone is context, not the content. */
  tone?: "neutral" | "good" | "warn" | "bad";
  /** What shape this card is. `facts` is the original and stays the default.
   *
   *  Three more, because three results kept being forced into a shape that hid them: a
   *  list of checks (what ran, what passed), a row of figures with how each moved, and a
   *  single thing worth saying on its own. Same four slots, same read-only card. */
  kind?: "facts" | "checklist" | "metrics" | "notice";
  facts: Fact[];
  /** Where the facts came from — required of the agent and checked against what its
   *  run actually read, same as a chart's or a table's. */
  source?: string;
};

const TONES = new Set(["neutral", "good", "warn", "bad"]);

/** Validate an already-parsed value. Null for anything we cannot draw, so the caller
 *  falls back to the raw JSON rather than an empty frame. */
const KINDS = new Set(["facts", "checklist", "metrics", "notice"]);

export function asCardSpec(v: unknown): CardSpec | null {
  if (!v || typeof v !== "object") return null;
  const c = v as CardSpec;
  const kind = KINDS.has(c.kind ?? "") ? (c.kind as CardSpec["kind"]) : "facts";
  // A notice is its sentence; everything else is its facts.
  if (kind === "notice") {
    const note = typeof c.note === "string" ? c.note.trim() : "";
    return note ? { ...c, kind, facts: [], note, tone: TONES.has(c.tone ?? "") ? c.tone : "neutral" } : null;
  }
  if (!Array.isArray(c.facts) || c.facts.length === 0) return null;
  // A fact needs both halves: a label with no value says nothing, and a value with no
  // label is a number with no meaning — which is the failure a card exists to avoid.
  const facts = c.facts.filter(
    (f) => f && typeof f.label === "string" && f.label.trim() && f.value !== undefined && f.value !== null,
  );
  if (facts.length === 0) return null;
  return { ...c, kind, facts, tone: TONES.has(c.tone ?? "") ? c.tone : "neutral" };
}

const TONE_ICON = { good: "check", warn: "clock", bad: "shield", neutral: "diamond" } as const;

export function FactCard({ spec }: { spec: CardSpec }) {
  const tone = spec.tone ?? "neutral";
  return (
    <section className="factcard" data-tone={tone} data-testid="factcard">
      {spec.title && (
        <h4 className="factcard-title">
          <Icon name={TONE_ICON[tone]} size={ICON.md} />
          {spec.title}
        </h4>
      )}
      {spec.kind === "checklist" ? (
        <ul className="factcard-checks" data-testid="factcard-checks">
          {spec.facts.map((f, i) => (
            <li className="factcard-check" data-done={f.done ? "yes" : "no"} key={`${f.label}-${i}`}>
              <Icon name={f.done ? "check" : "x"} size={ICON.md} />
              <span className="factcard-check-label">{f.label}</span>
              <span className="factcard-check-value">{String(f.value)}</span>
            </li>
          ))}
        </ul>
      ) : (
        <dl className={"factcard-facts" + (spec.kind === "metrics" ? " is-metrics" : "")}>
          {spec.facts.map((f, i) => (
            <div className={"factcard-fact" + (f.emphasis ? " is-lead" : "")} key={`${f.label}-${i}`}>
              <dt>{f.label}</dt>
              <dd>
                {String(f.value)}
                {/* How it moved, said beside the figure rather than coloured: a red
                    number is unreadable at this size, and "down" is not always bad. */}
                {spec.kind === "metrics" && f.delta && <span className="factcard-delta">{f.delta}</span>}
              </dd>
            </div>
          ))}
        </dl>
      )}
      {spec.note && <p className="factcard-note">{spec.note}</p>}
      {spec.source && <p className="factcard-source">Source: {spec.source}</p>}
    </section>
  );
}
