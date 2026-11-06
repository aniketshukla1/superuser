import type { ConversationMessage } from "../api";
import type { Attachment, Item } from "../types";

export { mergeJournalSteps } from "./mergeJournalSteps";

type ToolItem = Extract<Item, { kind: "tool" }>;

/** A message's time in epoch seconds. The store writes `at`; older messages used `ts`.
 *  Either is accepted, or the time would quietly vanish and the transcript have no dates. */
export const stamp = (m: any): number | undefined => {
  if (typeof m?.ts === "number") return m.ts;
  return typeof m?.at === "number" ? m.at : undefined;
};

/** What the tool messages said, by tool_call_id, so a replayed tool row can show its output
 *  (live, `tool_finished` brings it). `hidden` counts what the privacy filters removed, from
 *  the message's `_display` record, which the agent never saw. */
function toolOutputs(messages: ConversationMessage[]) {
  const previews: Record<string, string> = {};
  const hidden: Record<string, number> = {};
  for (const m of messages) {
    if (m.role !== "tool" || !m.tool_call_id) continue;
    previews[m.tool_call_id] = typeof m.content === "string" ? m.content : JSON.stringify(m.content);
    const removed = Number(m._display?.hidden_by_filters || 0);
    if (removed > 0) hidden[m.tool_call_id] = removed;
  }
  return { previews, hidden };
}

function userItems(m: ConversationMessage, out: Item[]) {
  // A connector message becomes a structured card: the wrapped `content` is for the model,
  // and the card draws from the `source` record.
  if (m.source?.connector) {
    out.push({ kind: "connector", source: m.source });
    return;
  }
  const user = userItemFromContent(m.content);
  // A forced skill ("/skill …"): `_display` is the line the user typed, `content` the framing
  // for the model. The bubble shows what was typed.
  if (typeof m._display === "string" && m._display) user.text = m._display;
  // `ts`, unix seconds, is the server's stamp; older sessions have none.
  const time = stamp(m);
  if (time !== undefined) user.ts = time;
  if (user.text || user.attachments?.length) out.push(user);
}

// An assistant message gives the WORK first and the answer after it: the order the turn
// happened in, and the order `Transcript` needs. When the text came first, every reopened
// conversation in which a tool was used hid its own answer: `Transcript.flush` finds the
// answer by taking TRAILING assistant items off the turn, and text ahead of the tools never
// trailed, so it folded into the collapsed "N steps" (a chart came back as "› 1 step" and a
// chart, with the explanation hidden). Live streaming was fine, which made it hard to see; it
// only showed after switching away and back (owner, 2026-09-01).
function assistantItems(m: ConversationMessage, out: Item[], outputs: ReturnType<typeof toolOutputs>) {
  const saved: any[] = (m as any).items || [];
  // The sub-threads this turn fanned out into, saved on the message by the socket.
  const threads = saved
    .filter((it) => it?.kind === "thread" && it.session_id)
    .map((it) => ({ session_id: String(it.session_id), title: String(it.title || "Thread") }));
  if (threads.length > 0) out.push({ kind: "threads", threads });
  // Calls saved on the message (charts, for now) and the browser's kept screens, in the order
  // the turn made them. The store doesn't keep a turn's tool calls, but a chart that vanishes
  // when you switch chats is worse than none, since the answer goes on referring to it.
  for (const it of saved) {
    if (it?.kind === "computer" && it.persona && it.frame) {
      out.push({ kind: "computer", persona: String(it.persona), frame: String(it.frame), url: String(it.url ?? ""), title: String(it.title ?? "") });
    } else if (it?.kind === "tool" && it.name) {
      out.push({ kind: "tool", id: `${it.name}-${out.length}`, name: String(it.name), args: it.args ?? {}, status: String(it.status || "ok") });
    }
  }
  for (const call of m.tool_calls || []) {
    let args: any = {};
    try {
      args = JSON.parse(call.function?.arguments || "{}");
    } catch {
      args = {};
    }
    const row: ToolItem = { kind: "tool", id: call.id, name: call.function?.name, args, status: "ok", preview: outputs.previews[call.id] };
    const removed = outputs.hidden[call.id];
    if (removed) row.hidden = removed;
    out.push(row);
  }
  if (!m.content && !m.reasoning) return;
  const answer: Extract<Item, { kind: "assistant" }> = { kind: "assistant", text: m.content || "" };
  const time = stamp(m);
  if (time !== undefined) answer.ts = time;
  if (m.reasoning) answer.reasoning = m.reasoning;
  if (m.persona) answer.persona = m.persona;
  out.push(answer);
}

/** A saved marker (the engine's `_append_notice`), so an error, an interruption or a model
 *  switch reads after a reload just as it did live. An error can be retried; the Transcript
 *  only offers that when it is the last thing in the transcript. */
function noticeItem(m: ConversationMessage): Item {
  switch (m.kind) {
    case "interrupted":
      return { kind: "notice", tone: "warn", text: "Interrupted." };
    case "model_switch":
      return { kind: "notice", tone: "info", text: m.text || "Model switched" };
    case "compacted":
      // The quiet "compacted here" divider (OPE-27); the transcript itself is whole.
      return { kind: "notice", tone: "info", text: m.text || "Context compacted" };
    default:
      return { kind: "notice", tone: "warn", text: "Error: " + (m.text || "unknown"), retriable: true };
  }
}

/** The GUI's items for a saved transcript. System messages are left out, and tool results are
 *  folded into their tool rows. */
export function itemsFromMessages(messages: ConversationMessage[]): Item[] {
  const all = messages || [];
  const outputs = toolOutputs(all);
  const out: Item[] = [];
  for (const m of all) {
    if (m.role === "user") userItems(m, out);
    else if (m.role === "assistant") assistantItems(m, out, outputs);
    else if (m.role === "notice") out.push(noticeItem(m));
  }
  return out;
}

/** A user message's content as an item: plain text, or text parts joined by blank lines with
 *  any inline images as attachments. */
export function userItemFromContent(content: any): Extract<Item, { kind: "user" }> {
  if (typeof content === "string") return { kind: "user", text: content };
  if (!Array.isArray(content)) return { kind: "user", text: "" };
  const paragraphs: string[] = [];
  const attachments: Attachment[] = [];
  for (const part of content) {
    if (part === null || typeof part !== "object") continue;
    if (part.type === "text" && part.text) {
      paragraphs.push(String(part.text));
      continue;
    }
    const url = part.type === "image_url" ? part.image_url?.url : undefined;
    if (typeof url === "string" && url.startsWith("data:image/")) attachments.push({ kind: "image", name: "image", data_url: url });
  }
  return { kind: "user", text: paragraphs.join("\n\n"), attachments };
}
