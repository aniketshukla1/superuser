import { fetch, httpBase, jsonOf } from "./neverRejectingJson";

// -- The run journal: what a turn actually did --------------------------------

export interface RunEvent {
  /** Line number in the journal — also the replay offset. */
  seq: number;
  type: string;
  at: number;
  /** Whatever that frame carried. The journal records frames as they pass rather
   *  than imposing a schema, so the shape varies by `type`. */
  data?: Record<string, unknown>;
}

export interface RunJournal {
  events: RunEvent[];
  next_offset: number;
  total: number;
  /** The file hit its cap and older lines were dropped. */
  truncated: boolean;
  running: boolean;
}

export async function getRunJournal(sessionId: string, offset = 0, limit = 200): Promise<RunJournal> {
  const res = await fetch(
    `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/run?offset=${offset}&limit=${limit}`,
  );
  if (!res.ok) return { events: [], next_offset: offset, total: 0, truncated: false, running: false };
  const j = await jsonOf(res);
  return {
    events: j.events ?? [],
    next_offset: j.next_offset ?? offset,
    total: j.total ?? 0,
    truncated: j.truncated ?? false,
    running: j.running ?? false,
  };
}
