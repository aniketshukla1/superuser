import type { Item, WsEvent } from "../../../types";
import { FILE_WRITE_TOOLS, newId, normalizeTodos, updateLastTool } from "../../appHelpers";
import type { SocketCtx } from "./handleEvent";

type Data = WsEvent["data"];

/** Requests that an Unattended session parks in the Inbox; no live card is shown for them. */
const PARKED_WHEN_UNATTENDED = new Set(["permission_required", "directory_requested", "plan_proposed"]);

const text = (value: unknown, fallback = "") => String(value || fallback);

/** The transcript item an event adds, or null when it adds none. */
function itemFor(type: WsEvent["type"], d: Data, agent: string): Item | null {
  switch (type) {
    case "tool_proposed":
      return { kind: "tool", id: newId(), name: d.name, args: d.arguments, status: "…" };
    case "permission_required":
      return { kind: "approval", name: d.name, args: d.arguments, reason: d.reason, category: d.category, standingTarget: d.standing_target || undefined };
    case "directory_requested":
      return { kind: "dirreq", reason: d.reason || "", path: d.path || "", writable: Boolean(d.writable) };
    case "plan_proposed":
      return { kind: "planreq", plan: d.plan || "" };
    case "question_requested":
      // ask_user in an attended session, answered right here rather than in the Inbox.
      return { kind: "question", questions: Array.isArray(d.questions) ? d.questions : [] };
    case "form_requested":
      // ask_form: the same hand-off as a question, with a different card.
      return { kind: "form", form: { title: d.title, intro: d.intro, fields: Array.isArray(d.fields) ? d.fields : [] } };
    // The assistant's own browser. A kept screen joins the turn; the two requests hold the turn
    // until they are answered at the head of the composer, as a question does.
    case "computer_frame":
      return d.persona && d.frame ? { kind: "computer", persona: text(d.persona), frame: text(d.frame), url: text(d.url), title: text(d.title) } : null;
    case "computer_help":
      return { kind: "computer_help", persona: text(d.persona, agent), reason: text(d.reason), url: text(d.url) };
    case "computer_secret":
      return { kind: "computer_secret", persona: text(d.persona, agent), label: text(d.label, "a value") };
    default:
      return null;
  }
}

/** What the agent asks for or reports during a turn: tools, permissions, folders, plans,
 *  questions, forms and its browser. */
export function onRequestEvent(c: SocketCtx, ev: WsEvent, d: Data, keepPartial: () => void) {
  if (ev.type === "tool_finished") {
    c.setItems((before) => updateLastTool(before, d.name, d.status, d.result_preview || d.reason, d.display?.hidden_by_filters, d.standing_rule));
    // The panel refreshes when what it shows may have changed: the browser, or a written file
    // that belongs under Artifacts now rather than when the turn ends.
    if (String(d.name || "").startsWith("browser_") || FILE_WRITE_TOOLS.has(d.name)) c.setBrowserRefreshKey((key) => key + 1);
    return;
  }
  if (ev.type === "tool_proposed") {
    // What the model said before reaching for a tool is finished text, so it becomes a
    // narration row in this turn's group. Local models began streaming once `stream: false`
    // was lifted, and without this their narration stayed in the live buffer and ran into the
    // next step's text.
    keepPartial();
    const todos = d.arguments?.todos || d.arguments?.items;
    if (d.name === "todo_write" && todos) c.setTodo(normalizeTodos(d.arguments.todos ?? d.arguments.items));
  }
  // Unattended: the server put it in the Inbox, so there's no live card as well.
  if (PARKED_WHEN_UNATTENDED.has(ev.type) && c.unattendedRef.current) return;
  const item = itemFor(ev.type, d, c.agent);
  if (item) c.setItems((before) => [...before, item]);
}
