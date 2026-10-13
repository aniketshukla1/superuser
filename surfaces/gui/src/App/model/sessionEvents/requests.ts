import { normalizeTodos, newId, updateLastTool, FILE_WRITE_TOOLS } from "../../appHelpers";
import type { WsEvent } from "../../../types";
import type { SocketCtx } from "./handleEvent";

/** What the agent asks for or reports mid-turn: tools, permissions, folders, plans, questions, the browser. */
export function onRequestEvent(c: SocketCtx, ev: WsEvent, d: WsEvent["data"], flushPartialStream: () => void) {
  const { agent, setItems, setTodo, unattendedRef, setBrowserRefreshKey } = c;
  switch (ev.type) {
    case "tool_proposed":
      // Whatever the model said before reaching for a tool is finished text: promote it
      // to a narration row in this turn's group. Local models only started streaming
      // once `stream: false` was lifted, and without this their per-step narration
      // stayed in the live buffer and got glued onto the next step's text.
      flushPartialStream();
      if (d.name === "todo_write" && (d.arguments?.todos || d.arguments?.items))
        setTodo(normalizeTodos(d.arguments.todos ?? d.arguments.items));
      setItems((p) => [
        ...p,
        { kind: "tool", id: newId(), name: d.name, args: d.arguments, status: "…" },
      ]);
      break;
    case "permission_required":
      // Unattended → the backend parked it in the Inbox; don't also surface a live card.
      if (unattendedRef.current) break;
      setItems((p) => [
        ...p,
        {
          kind: "approval",
          name: d.name,
          args: d.arguments,
          reason: d.reason,
          category: d.category,
          standingTarget: d.standing_target || undefined,
        },
      ]);
      break;
    case "directory_requested":
      if (unattendedRef.current) break;
      setItems((p) => [
        ...p,
        { kind: "dirreq", reason: d.reason || "", path: d.path || "", writable: !!d.writable },
      ]);
      break;
    case "plan_proposed":
      if (unattendedRef.current) break;
      setItems((p) => [...p, { kind: "planreq", plan: d.plan || "" }]);
      break;
    case "question_requested":
      // ask_user in an attended session — answered inline (not routed to the Inbox).
      setItems((p) => [...p, { kind: "question", questions: Array.isArray(d.questions) ? d.questions : [] }]);
      break;
    // The assistant's own browser. A kept screen joins the turn; the two requests block
    // it until answered in the composer head, like a question does.
    case "computer_frame":
      if (d.persona && d.frame)
        setItems((p) => [
          ...p,
          { kind: "computer", persona: String(d.persona), frame: String(d.frame), url: String(d.url || ""), title: String(d.title || "") },
        ]);
      break;
    case "computer_help":
      setItems((p) => [
        ...p,
        { kind: "computer_help", persona: String(d.persona || agent), reason: String(d.reason || ""), url: String(d.url || "") },
      ]);
      break;
    case "computer_secret":
      setItems((p) => [...p, { kind: "computer_secret", persona: String(d.persona || agent), label: String(d.label || "a value") }]);
      break;
    case "form_requested":
      // ask_form: the same rendezvous, a different card.
      setItems((p) => [
        ...p,
        { kind: "form", form: { title: d.title, intro: d.intro, fields: Array.isArray(d.fields) ? d.fields : [] } },
      ]);
      break;
    case "tool_finished":
      setItems((p) =>
        updateLastTool(
          p,
          d.name,
          d.status,
          d.result_preview || d.reason,
          d.display?.hidden_by_filters,
          d.standing_rule,
        ),
      );
      // Refresh the right rail when something it shows may have changed: browser state, or a
      // file write that should appear under Artifacts immediately (not only after the turn).
      if (String(d.name || "").startsWith("browser_") || FILE_WRITE_TOOLS.has(d.name)) {
        setBrowserRefreshKey((k) => k + 1);
      }
      break;
  }
}
