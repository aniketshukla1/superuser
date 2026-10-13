import type { Item } from "../../types";

type Threads = Extract<Item, { kind: "threads" }>["threads"];

/** The sub-threads a turn fanned out into, reachable from the conversation that made them. */
export function ThreadLinks({ threads, onOpenThread }: { threads: Threads; onOpenThread?: (id: string) => void }) {
  return (
    <div className="threadlinks">
      {threads.map((t) => (
        <button
          key={t.session_id}
          className="threadlink"
          onClick={() => onOpenThread?.(t.session_id)}
          title={t.title}
        >
          <span className="threadlink-arrow">└</span>
          <span className="threadlink-title">{t.title}</span>
          <span className="threadlink-go">open →</span>
        </button>
      ))}
    </div>
  );
}
