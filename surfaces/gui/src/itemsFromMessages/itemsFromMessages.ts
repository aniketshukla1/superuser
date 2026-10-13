import type { ConversationMessage } from "../api";
import type { ToolCall } from "../components/ActivityTab";
import type { Attachment, Item } from "../types";

/** Our session store persists `at` (epoch seconds); older messages used `ts`.
 *  Accept either, or the timestamp is silently lost and the transcript has no dates. */
export const stamp = (m: any): number | undefined =>
  typeof m?.ts === "number" ? m.ts : typeof m?.at === "number" ? m.at : undefined;

export function itemsFromMessages(messages: ConversationMessage[]): Item[] {
  const items: Item[] = [];
  // Index tool results by tool_call_id so replayed tool rows can show their output
  // (the live view gets this from `tool_finished` events; on replay it's the `role:"tool"` msgs).
  const results: Record<string, string> = {};
  // `_display` sidecar on a tool message = user-facing metadata the agent never saw
  // (e.g. how many hits the privacy filters hid) — surfaces on the tool card.
  const hiddenCounts: Record<string, number> = {};
  for (const m of messages || []) {
    if (m.role === "tool" && m.tool_call_id) {
      results[m.tool_call_id] =
        typeof m.content === "string" ? m.content : JSON.stringify(m.content);
      const hidden = Number(m._display?.hidden_by_filters || 0);
      if (hidden > 0) hiddenCounts[m.tool_call_id] = hidden;
    }
  }
  for (const m of messages || []) {
    if (m.role === "user") {
      // Connector message → structured card; the framed `content` stays for the model, but display
      // renders from the source sidecar.
      if (m.source?.connector) {
        items.push({ kind: "connector", source: m.source });
        continue;
      }
      const user = userItemFromContent(m.content);
      // Force-run (`/skill …`): `_display` holds the user's literal line; `content` carries
      // the model-facing framing. Render what the user typed — one truthful bubble.
      if (typeof m._display === "string" && m._display) user.text = m._display;
      // `ts` (unix seconds) is the server's canonical-message stamp; older sessions have none.
      { const t = stamp(m); if (t !== undefined) user.ts = t; }
      if (user.text || user.attachments?.length) items.push(user);
    } else if (m.role === "assistant") {
      // The WORK first, then the answer — the order the turn actually happened in, and
      // the order `Transcript` requires.
      //
      // This used to push the text first, and the effect was that every reopened
      // conversation in which the assistant had used a tool hid its own answer.
      // `Transcript.flush` identifies the final answer positionally, by popping
      // TRAILING assistant items off the turn; with the text emitted ahead of the
      // tools it never trailed, so it was folded into the collapsed "N steps"
      // disclosure instead of rendering as a bubble. A turn that drew a chart came
      // back as "› 1 step" and a chart, with the explanation of both hidden behind a
      // triangle — and `Transcript` says in as many words that the intended order is
      // the chart is drawn, then talked about.
      //
      // Live streaming was never affected, which is what made this hard to see: it
      // looked right until you switched away and came back. (Owner-hit 2026-09-01 on
      // an AI-trends chart whose 1,077-character answer was on disk the whole time.)

      // Fan-out record persisted on the assistant message by the socket.
      const threads = ((m as any).items || [])
        .filter((it: any) => it?.kind === "thread" && it.session_id)
        .map((it: any) => ({ session_id: String(it.session_id), title: String(it.title || "Thread") }));
      if (threads.length) items.push({ kind: "threads", threads });
      // Tool calls persisted on the message (charts, today). The store does NOT keep the
      // turn's tool calls, so a reopened conversation has no steps at all — but a chart
      // that only exists until you switch chats is worse than one that was never drawn,
      // because the answer above it goes on referring to it.
      for (const it of (m as any).items || []) {
        // A screen the assistant's browser kept, in the order the turn produced it —
        // charts and screens share one list on the message for exactly that reason.
        if (it?.kind === "computer" && it.persona && it.frame) {
          items.push({
            kind: "computer",
            persona: String(it.persona),
            frame: String(it.frame),
            url: String(it.url ?? ""),
            title: String(it.title ?? ""),
          });
          continue;
        }
        if (it?.kind !== "tool" || !it.name) continue;
        items.push({
          kind: "tool",
          id: `${it.name}-${items.length}`,
          name: String(it.name),
          args: it.args ?? {},
          status: String(it.status || "ok"),
        });
      }
      for (const tc of m.tool_calls || []) {
        let args: any = {};
        try {
          args = JSON.parse(tc.function?.arguments || "{}");
        } catch {
          args = {};
        }
        const preview = results[tc.id];
        const hidden = hiddenCounts[tc.id];
        items.push({
          kind: "tool",
          id: tc.id,
          name: tc.function?.name,
          args,
          status: "ok",
          preview,
          ...(hidden ? { hidden } : {}),
        });
      }
      if (m.content || m.reasoning)
        items.push({
          kind: "assistant",
          text: m.content || "",
          ...(stamp(m) !== undefined ? { ts: stamp(m)! } : {}),
          ...(m.reasoning ? { reasoning: m.reasoning } : {}),
          ...(m.persona ? { persona: m.persona } : {}),
        });
    } else if (m.role === "notice") {
      // Persisted markers (engine `_append_notice`): error/interrupted/model-switch survive
      // reload exactly like the live view rendered them. An error notice is retriable —
      // the Transcript only offers the button when it's the transcript tail.
      items.push(
        m.kind === "interrupted"
          ? { kind: "notice", tone: "warn", text: "Interrupted." }
          : m.kind === "model_switch"
            ? { kind: "notice", tone: "info", text: m.text || "Model switched" }
            : m.kind === "compacted"
              ? // The subtle "compacted here" divider (OPE-27) — the transcript itself is intact.
                { kind: "notice", tone: "info", text: m.text || "Context compacted" }
              : { kind: "notice", tone: "warn", text: "Error: " + (m.text || "unknown"), retriable: true },
      );
    }
    // system messages are omitted; tool-result messages are folded into the tool row above
  }
  return items;
}

/** Put the last turn's steps back into a reopened conversation.
 *
 *  The session store keeps the answer and not the working (see the note on tool calls
 *  above), so a conversation you switched away from and back to read as though the
 *  model had simply known: "The files in the persistent working directory are:
 *  alpha.md" under a request to run a shell command, with nothing between the question
 *  and the answer. The working DID survive — the run journal keeps every frame of the
 *  most recent turn (`core::runlog`) — it just had only one reader, the side panel.
 *
 *  So the journal's calls are placed where they happened: after the last thing the user
 *  sent, ahead of the reply and any notice that followed it. Only the last turn, because
 *  that is all the journal holds; older turns stay as they are rather than borrowing
 *  steps that were not theirs. A turn that already carries steps (persisted charts, or
 *  an older history with `tool_calls`) is left alone, so nothing renders
 *  twice.
 *
 *  A call the journal never saw finish belongs to a run that is over — the caller only
 *  merges an idle journal — so it is marked stopped, not left on a spinner. */
export function mergeJournalSteps(items: Item[], calls: ToolCall[]): Item[] {
  if (!calls.length) return items;
  let lastAsk = -1;
  for (let i = items.length - 1; i >= 0; i--) {
    const k = items[i].kind;
    if (k === "user" || k === "connector") {
      lastAsk = i;
      break;
    }
  }
  const tail = items.slice(lastAsk + 1);
  if (tail.some((it) => it.kind === "tool")) return items;
  // Nothing after the question is a turn that never answered, or a question still being
  // answered: either way there is no reply for these steps to sit in front of.
  if (lastAsk >= 0 && tail.length === 0) return items;
  const steps: Item[] = calls.map((c) => ({
    kind: "tool",
    id: `journal-${c.seq}`,
    name: c.name,
    args: c.args,
    status: c.pending || c.stopped ? "stopped" : c.status || "ok",
    ...(c.result ? { preview: c.result } : {}),
  }));
  return [...items.slice(0, lastAsk + 1), ...steps, ...tail];
}

export function userItemFromContent(content: any): Extract<Item, { kind: "user" }> {
  if (typeof content === "string") return { kind: "user", text: content };
  if (!Array.isArray(content)) return { kind: "user", text: "" };

  const text: string[] = [];
  const attachments: Attachment[] = [];
  for (const part of content) {
    if (!part || typeof part !== "object") continue;
    if (part.type === "text" && part.text) {
      text.push(String(part.text));
    } else if (part.type === "image_url") {
      const url = part.image_url?.url;
      if (typeof url === "string" && url.startsWith("data:image/")) {
        attachments.push({ kind: "image", name: "image", data_url: url });
      }
    }
  }
  return { kind: "user", text: text.join("\n\n"), attachments };
}
