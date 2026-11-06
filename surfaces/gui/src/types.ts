// Shapes shared across the GUI: the session socket's events, sessions, attachments, usage, and
// the items a transcript is made of. The wire contract itself lives in api.ts; this file takes
// a type from it but never a value, so the two have no runtime cycle.
import type { MessageSource } from "./api";

/** Every event the session socket can send, alphabetically. */
export type EventType =
  | "assistant_delta"
  | "assistant_message"
  | "compacted"
  | "compacting"
  // The assistant's own browser: a screen kept after a navigation, and the two things a turn
  // can wait on, the user's hands or one value the assistant must never see.
  | "computer_frame"
  | "computer_help"
  | "computer_secret"
  | "directory_requested"
  | "error"
  | "form_requested"
  | "inbound"
  | "input_rejected"
  | "interrupted"
  | "iteration_end"
  // The permission mode changed, not the model. Kept apart because the GUI writes a notice
  // for `model_changed`, and that notice must not claim the model changed when it hadn't.
  | "mode_changed"
  | "model_changed"
  | "permission_required"
  | "plan_proposed"
  | "question_requested"
  | "ready"
  | "reasoning_delta"
  // A message sent while a turn was running: accepted and queued for the agent's next step,
  // neither dropped nor starting a second turn.
  | "steered"
  | "tool_finished"
  | "tool_proposed"
  | "tool_started"
  | "turn_done"
  | "turn_end"
  | "turn_start"
  // The turn's running token total, one per model call; it drives the status line under the
  // transcript while the turn works.
  | "usage";

export interface WsEvent {
  data: any;
  type: EventType;
}

/** How an approval was answered. `always_task` is saved on the automation that owns the run
 *  (a standing scoped approval, UX-DECISIONS §25) and is offered only on cards from
 *  automation runs, in the app. */
export type ApprovalDecision = "always_command" | "always_task" | "always_tool" | "deny" | "once";

export interface TodoItem {
  status: "done" | "in_progress" | "pending";
  content: string;
}

/** Token counts for one request. The server adds them to each assistant message and to the
 *  assistant_message event; servers from before that, and backends that don't count, omit them. */
export interface TurnUsage {
  model?: string | null;
  cache_read: number;
  cache_write: number;
  input: number;
  output: number;
}

/** A session's totals by model id (more than one when the model changed partway). `context`
 *  is the prompt side of the latest request: what fills the active model's window now. */
export interface SessionUsage {
  context: number;
  byModel: Record<string, TurnUsage>;
}

export interface SessionInfo {
  session_id: string;
  title?: string;
  /** True when the title was chosen rather than guessed from the first message. The sidebar
   *  wants the guess; the chat's own heading does not. */
  named?: boolean;
  /** The opening message, for the row's tooltip: the name is a handle, not a summary. */
  preview?: string;
  agent: string;
  model: string;
  mode: string;
  workspace: string;
  messages: number;
  updated_at: string | null;
  pinned?: boolean;
  archived?: boolean;
  /** Inbox items waiting on this session: the amber count that rises up the sidebar. */
  attention?: number;
  /** A dot without a count: `working` while a turn runs, `sleeping` while a self-wake is
   *  pending, `idle` otherwise. */
  liveness?: "idle" | "sleeping" | "working";
  /** The channels this session listens to. */
  subscriptions?: string[];
  /** §31: present when a mention on some platform opened the session instead of the user: a
   *  machine key ("slack") and a label ("#general · T0ABCD"), which decide the sidebar's
   *  "From Slack" group and the icon on the row. */
  origin?: string;
  origin_label?: string;
  /** On a sub-thread, the session it was split from. A request written as a list fans out
   *  into one child session per task, listed under its parent. */
  parent_id?: string | null;
}

/** A file sent with a user message: images and PDFs as data URLs, text files inline. */
export interface Attachment {
  kind: "image" | "pdf" | "text";
  name: string;
  mime?: string;
  data_url?: string;
  text?: string;
}

// The transcript, item by item. `ts` is in unix seconds: the server's stamp on a stored
// message, or the local clock for a live one. Sessions saved before the server stamped
// messages have none.

type UserItem = { kind: "user"; text: string; attachments?: Attachment[]; ts?: number };
/** A message that came in through a connector (Slack, Salesforce, …), drawn as a structured
 *  card (ConnectorMessageCard) instead of a plain bubble. The connector registry decides how
 *  each one looks, so no connector needs code of its own here. */
type ConnectorItem = { kind: "connector"; source: MessageSource };
/** `persona` names who answered, when the person addressed another assistant with `@id`. */
type AssistantItem = { kind: "assistant"; text: string; ts?: number; reasoning?: string; persona?: string };
/** The sub-threads a turn fanned out into, linked under the reply so they can be reached from
 *  the conversation that made them, not only from the sidebar. */
type ThreadsItem = { kind: "threads"; threads: { session_id: string; title: string }[] };
/** `hidden` is how many results the person's privacy filters took out before the agent saw any
 *  (read from the tool message's `_display` record; the agent's copy shows no sign of them).
 *  `standingRule` names the task-scoped rule that allowed the call ("tool → target"). */
type ToolItem = {
  kind: "tool";
  status: string;
  id: string;
  name: string;
  args: any;
  standingRule?: string;
  preview?: string;
  hidden?: number;
};
type ApprovalItem = {
  kind: "approval";
  resolved?: ApprovalDecision;
  reason: string;
  name: string;
  args: any;
  category?: string;
  /** Where a standing rule would be pinned, as the server computes it. When the approval
   *  comes from a run, the card can offer "Allow every time" (§25). */
  standingTarget?: string;
};
type DirectoryRequestItem = { kind: "dirreq"; reason: string; path?: string; writable?: boolean; resolved?: "granted" | "denied" };
type PlanRequestItem = { kind: "planreq"; plan: string; resolved?: "approved" | "rejected" };
/** A live `ask_user` prompt: the agent's questions one at a time, each with the answers it
 *  expects. Answered inline in an attended session; sent to the Inbox in an unattended one. */
type QuestionItem = {
  kind: "question";
  questions: { question: string; options: { label: string; description?: string }[]; multi?: boolean }[];
  resolved?: string;
};
type FormField = { name: string; label: string; kind?: "text" | "number" | "date"; placeholder?: string; help?: string; required?: boolean };
/** A live `ask_form` prompt: values only the user holds. A `question` is for a decision and
 *  carries options; this carries fields. */
type FormItem = { kind: "form"; form: { title?: string; intro?: string; fields: FormField[] }; resolved?: string };
type NoticeItem = { kind: "notice"; tone: "info" | "warn"; text: string; retriable?: boolean };
/** A screen the assistant's browser kept when it opened a page: how the page looked then, so
 *  a finished turn never shows today's screen in its place. */
type ComputerItem = { kind: "computer"; persona: string; frame: string; url: string; title: string };
/** The browser waits for the user's hands (a sign-in, a CAPTCHA). */
type ComputerHelpItem = { kind: "computer_help"; persona: string; reason: string; url: string; resolved?: "taken" | "declined" | "ended" };
/** The browser waits for one value, typed straight into a field the assistant never sees. */
type ComputerSecretItem = { kind: "computer_secret"; persona: string; label: string; resolved?: "sent" | "declined" | "ended" };

export type Item =
  | UserItem
  | ConnectorItem
  | AssistantItem
  | ThreadsItem
  | ToolItem
  | ApprovalItem
  | DirectoryRequestItem
  | PlanRequestItem
  | QuestionItem
  | FormItem
  | NoticeItem
  | ComputerItem
  | ComputerHelpItem
  | ComputerSecretItem;
