import type { SessionUsage } from "../../types";
import { formatTokens, totalTokens } from "../../usage";
import { shortModel } from "./Props";
import { usageStat } from "./SlashRow";

/** The per-model token table the old usage popover carried. It did not stop being
 *  useful when the trigger became a ring; it stopped being the FIRST thing you see.
 *  Built here rather than inside the ring because only the composer knows the labels. */
export function UsageBreakdown({ usage, modelLabels }: { usage: SessionUsage; modelLabels?: Record<string, string> }) {
  return (
    <div className="flex flex-col gap-1.5" data-testid="usage-by-model">
      {Object.entries(usage.byModel).map(([id, t]) => (
        <div key={id}>
          <div className="composer-menu-name truncate" title={id}>
            {id === "unknown" ? "Unknown model" : modelLabels?.[id] || shortModel(id)}
          </div>
          {/* With a cache split the input rows are the three BILLING CLASSES of input,
              each priced differently, and they read as components: uncached + cache
              reads + cache writes = total. Without one, plain "Input" says it all. */}
          <div className="mt-0.5 flex flex-col gap-0.5">
            {t.cache_read + t.cache_write > 0 ? (
              <>
                {usageStat("Uncached input", t.input)}
                {usageStat("Cache reads", t.cache_read)}
                {usageStat("Cache writes", t.cache_write)}
                {usageStat("Total input", t.input + t.cache_read + t.cache_write)}
              </>
            ) : (
              usageStat("Input", t.input)
            )}
            {usageStat("Output", t.output)}
          </div>
        </div>
      ))}
      <div className="mt-1 pt-1.5 border-t border-line flex items-baseline justify-between">
        <span className="text-xs text-faint">Session total</span>
        <span className="text-xs text-ink tabular-nums">
          {formatTokens(totalTokens(usage))} tokens
        </span>
      </div>
    </div>
  );
}
