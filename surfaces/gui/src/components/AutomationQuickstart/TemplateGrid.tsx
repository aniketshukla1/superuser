import type { Connector } from "../../api";
import { ConnectorBadge } from "../../connectors/ConnectorIcon";
import { AVATAR } from "../Icon";
import { TEMPLATES, type QuickTemplate } from "./TEMPLATES";

/** The template cards. Equal-height (owner ask 2026-07-12): 1fr rows + h-full — <button>
 * grid items don't stretch like divs. */
export function TemplateGrid({
  pickedKey,
  connState,
  onPick,
}: {
  pickedKey: string | null;
  connState: (name: string) => Connector | undefined;
  onPick: (t: QuickTemplate) => void;
}) {
  return (
    <div className="grid grid-cols-3 auto-rows-fr gap-3">
      {TEMPLATES.map((t) => (
        <button
          key={t.key}
          data-testid={`qs-template-${t.key}`}
          className={
            "h-full text-left rounded-xl2 border bg-panel p-4 flex flex-col gap-1.5 " +
            (pickedKey === t.key
              ? "border-accent ring-2 ring-accentSoft"
              : "border-line hover:border-lineStrong")
          }
          onClick={() => onPick(t)}
        >
          <span className="text-md font-semibold">{t.title}</span>
          <span className="text-sm text-muted leading-prose flex-1">{t.blurb}</span>
          <span className="flex items-center gap-1.5 mt-1">
            {t.conns.map((c) => {
              const cs = connState(c.name);
              const on = !!cs?.connected;
              return (
                <span
                  key={c.name}
                  title={`${cs?.title || c.name} — ${on ? "connected" : "not connected yet"}`}
                  style={on ? undefined : { filter: "grayscale(1)", opacity: 0.55 }}
                >
                  {cs ? (
                    <ConnectorBadge connector={cs} size={AVATAR.sm} title={cs.title} />
                  ) : (
                    <span className="inline-block w-4 h-4 rounded-full border border-line2" />
                  )}
                </span>
              );
            })}
            <span className="text-xs text-faint ml-0.5">
              {t.conns.length === 0 ? `No connections needed · ${t.cadence}` : t.cadence}
            </span>
          </span>
        </button>
      ))}
    </div>
  );
}
