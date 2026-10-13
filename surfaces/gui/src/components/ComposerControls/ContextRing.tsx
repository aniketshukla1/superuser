import { useEffect, useState, type ReactNode } from "react";
import { getUsageRollup, type UsageRollup, type UsageWindow } from "../../api";
import { formatTokens } from "../../usage";
import { Menu, MenuDivider, MenuHeading } from "./MenuRow";
import { AttachRow } from "./AttachMenu";

/* ----------------------------------------------------------------- context */

export const pct = (used: number, of: number) => (of > 0 ? Math.min(100, Math.round((used / of) * 100)) : 0);

/** The ring. A circle reads as a gauge at 18px where a bar reads as a decoration, and
 *  this sits at the end of a row of words where a bar looked like an underline. */
export function Ring({ fill }: { fill: number }) {
  const r = 7;
  const c = 2 * Math.PI * r;
  return (
    <svg width={18} height={18} viewBox="0 0 18 18" aria-hidden="true" className="composer-ring">
      <circle cx="9" cy="9" r={r} className="composer-ring-track" />
      {/* Nothing at all at zero. A round line cap on a zero-length arc still paints a
          dot, and a dot at the top of an empty gauge reads as "a little bit used" —
          which is exactly the wrong thing to say when the window is unknown or the
          session has not started. */}
      {fill > 0 && (
        <circle
          cx="9"
          cy="9"
          r={r}
          className="composer-ring-fill"
          strokeDasharray={`${(c * Math.min(100, fill)) / 100} ${c}`}
          transform="rotate(-90 9 9)"
        />
      )}
    </svg>
  );
}

export function UsageBar({ title, window: w }: { title: string; window: UsageWindow }) {
  // A number, not a bar. There is no quota to be a fraction of: the bar was this window
  // against the busier of the two, so the busier one was always full — which reads as a
  // limit being reached. The countdown beside it ("Resets in 11 hr") said the same untrue
  // thing louder. What is true is the count, and when the window started.
  return (
    <div className="composer-usage-row" data-testid={`usage-${title.toLowerCase()}`}>
      <div className="composer-usage-head">
        <span className="composer-menu-name">{title}</span>
        <span className="composer-usage-total">{formatTokens(w.tokens)}</span>
      </div>
      {w.uncounted > 0 && (
        // Said, not rounded away: usage is optional per turn and missing means unknown.
        <div className="composer-menu-hint">
          {w.uncounted} of {w.counted + w.uncounted} turns didn’t report a cost, so this is a floor.
        </div>
      )}
    </div>
  );
}

export function ContextRing({
  contextUsed,
  contextWindow,
  breakdown,
}: {
  contextUsed: number;
  contextWindow?: number;
  /** The per-model table. Disclosed inside this panel rather than opening another
   *  surface: it is detail about what you are already looking at. */
  breakdown?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const [detail, setDetail] = useState(false);
  const [rollup, setRollup] = useState<UsageRollup | null>(null);
  const fill = contextWindow ? pct(contextUsed, contextWindow) : 0;

  useEffect(() => {
    if (!open) return;
    let live = true;
    void getUsageRollup().then((r) => live && setRollup(r));
    return () => {
      live = false;
    };
  }, [open]);
  useEffect(() => {
    if (!open) setDetail(false);
  }, [open]);

  return (
    <div className="composer-slot is-fixed">
      <button
        type="button"
        className={"composer-chip is-ring" + (open ? " is-open" : "")}
        onClick={() => setOpen((v) => !v)}
        title={contextWindow ? `Context window ${fill}% full` : "Usage"}
        aria-label="Context and usage"
        aria-haspopup="menu"
        aria-expanded={open}
        data-testid="context-ring"
      >
        <Ring fill={fill} />
      </button>
      {open && (
        <Menu onClose={() => setOpen(false)} label="Context and usage" align="right" width={330} testid="context-menu">
          <div className="composer-usage-row">
            <div className="composer-usage-head">
              <span className="composer-menu-name">Context window</span>
              <span className="composer-usage-total">
                {contextWindow
                  ? `${formatTokens(contextUsed)} / ${formatTokens(contextWindow)} (${fill}%)`
                  : formatTokens(contextUsed)}
              </span>
            </div>
            {contextWindow ? (
              <div className="composer-usage-track">
                <div className="composer-usage-fill" style={{ width: `${fill}%` }} />
              </div>
            ) : (
              <div className="composer-menu-hint">
                This model’s window isn’t known, so there is nothing to fill.
              </div>
            )}
          </div>
          <MenuDivider />
          {/* Not "plan usage": this app has no idea what plan you are on, and no provider
              tells it. This is what its own history can vouch for. */}
          <MenuHeading>Tokens through this app</MenuHeading>
          {rollup ? (
            <>
              <UsageBar title="Today" window={rollup.today} />
              <UsageBar title="This week" window={rollup.week} />
            </>
          ) : (
            <div className="composer-menu-hint" data-testid="usage-loading">Counting…</div>
          )}
          {breakdown && (
            <>
              <MenuDivider />
              <AttachRow
                icon="sliders"
                label={detail ? "Hide detailed breakdown" : "See detailed breakdown"}
                chevron
                onClick={() => setDetail((v) => !v)}
                testid="usage-breakdown"
              />
              {detail && <div className="composer-usage-row">{breakdown}</div>}
            </>
          )}
        </Menu>
      )}
    </div>
  );
}
