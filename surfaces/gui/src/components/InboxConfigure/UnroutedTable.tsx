import { useEffect, useState } from "react";
import { getUnrouted, type UnroutedItem } from "../../api";
import { CARD } from "../../ui";
import { exactTime, timeAgo } from "../../when";

// Dead-letter view: inbound messages that had no destination (e.g. a DM with no session designated)
// and background turns that failed (e.g. a dead model). Read-only — for visibility/debugging.
export function UnroutedTable() {
  const [items, setItems] = useState<UnroutedItem[] | null>(null);

  useEffect(() => {
    const load = () => getUnrouted().then(setItems).catch(() => setItems([]));
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, []);

  if (items && items.length === 0)
    return (
      <div className={CARD + " p-4 text-md text-muted"}>
        Nothing has gone undelivered.
      </div>
    );

  return (
    <div className={CARD + " overflow-hidden"}>
      <table className="w-full text-md">
        <thead className="text-xs text-faint">
          <tr className="text-left">
            <th className="font-medium px-4 py-2">When</th>
            <th className="font-medium px-4 py-2">Source</th>
            <th className="font-medium px-4 py-2">Reason</th>
            <th className="font-medium px-4 py-2">Message</th>
          </tr>
        </thead>
        <tbody>
          {(items ?? []).map((it, i) => (
            <tr className="border-t border-line" key={i}>
              <td className="px-4 py-2.5 text-muted whitespace-nowrap" title={exactTime(it.ts)}>
                {timeAgo(it.ts)}
              </td>
              <td className="px-4 py-2.5" title={it.sender}>
                {it.source}
              </td>
              <td className="px-4 py-2.5">
                <span className="text-warnInk" title={it.reason}>
                  {it.reason}
                </span>
              </td>
              <td className="px-4 py-2.5 text-muted truncate max-w-[16rem]" title={it.text}>
                {it.text}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
