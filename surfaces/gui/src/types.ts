export type EventType =
  | "ready"
  | "inbound"
  | "turn_start"
  | "assistant_delta"
  | "reasoning_delta"
  | "assistant_message"
  | "tool_proposed"
  | "permission_required"
  | "directory_requested"
  | "question_requested"
  | "form_requested"
  | "plan_proposed"
  | "tool_started"
  | "tool_finished"
  | "iteration_end"
  // Running token total for the turn, one per model call — drives the status line under
  // the transcript while it works.
  | "usage"
  | "turn_end"
  | "error"
  | "input_rejected"
  | "interrupted"
  // A message sent while a turn was in flight: accepted and queued for the
  // agent's next step, rather than dropped or starting a competing turn.
  | "steered"
  | "model_changed"
  // The permission mode moved, not the model. A separate kind because the GUI
  // writes a transcript notice for `model_changed`, and it must not claim the
  // model changed when it did not.
  | "mode_changed"
  | "compacting"
  | "compacted"
  // The assistant's own browser: a screen a navigation kept, and the two things a turn can
  // be blocked on — the user's hands, or one value the assistant must never see.
  | "computer_frame"
  | "computer_help"
  | "computer_secret"
  | "turn_done";

export interface WsEvent {
  type: EventType;
  data: any;
}

// Re-exported for transcript items below. Lives in api.ts (the REST/WS contract source of truth);
// type-only import, so there's no runtime cycle with api.ts's `import type { ... } from "./types"`.
import type { MessageSource } from "./api";

// "always_task" persists to the owning automation's task record (standing scoped
// approval, UX-DECISIONS §25) — offered only on automation-run approval cards, in-app.
export type ApprovalDecision = "once" | "deny" | "always_tool" | "always_command" | "always_task";

export interface TodoItem {
  content: string;
  status: "pending" | "in_progress" | "done";
}

// Per-round-trip token counts, as attached by the server to assistant messages and
// the assistant_message event (`{model, input, output, cache_read, cache_write}`).
// Absent on older servers and on backends that don't report usage.
export interface TurnUsage {
  model?: string | null;
  input: number;
  output: number;
  cache_read: number;
  cache_write: number;
}

// Per-session accumulation, keyed by model id (multiple models when the user
// switched mid-session). `context` = the latest round-trip's prompt-side total —
// what currently occupies the active model's context window.
export interface SessionUsage {
  byModel: Record<string, TurnUsage>;
  context: number;
}

export interface SessionInfo {
  session_id: string;
  title?: string;
  workspace: string;
  agent: string;
  model: string;
  mode: string;
  updated_at: string | null;
  messages: number;
  pinned?: boolean;
  archived?: boolean;
  // Inbox items awaiting this session (the amber attention count that bubbles up the sidebar).
  attention?: number;
  // working = in-flight turn; sleeping = a self-wake is pending; idle = neither. A count-less dot.
  liveness?: "working" | "sleeping" | "idle";
  // Channels this session listens to (inbound subscriptions).
  subscriptions?: string[];
  // §31: set when the session was spawned by a platform mention rather than the user —
  // machine key ("slack") + display label ("#general · T0ABCD"). Drives the sidebar's
  // "From Slack" group and the row's platform icon.
  origin?: string;
  origin_label?: string;
  // Set on a sub-thread: the session this one was split out of. One request written
  // as a list fans out into a child session per task, nested under its parent here.
  parent_id?: string | null;
  /** True when the title was chosen rather than guessed from the first message.
   *  The sidebar wants the guess; the chat's own heading does not. */
  named?: boolean;
  /** The opening message, for the row tooltip — the name is a handle, not a summary. */
  preview?: string;
}

// Attachments (images, PDFs, text files) sent with a user message.
export interface Attachment {
  kind: "image" | "text" | "pdf";
  name: string;
  mime?: string;
  data_url?: string; // images + PDFs
  text?: string; // text files
}

// Transcript items
// `ts` = unix seconds (the server's canonical-message stamp; live items stamp locally).
// Optional: sessions saved before the server stamped timestamps have none.
export type Item =
  | { kind: "user"; text: string; attachments?: Attachment[]; ts?: number }
  // A connector-delivered inbound message (Slack/Salesforce/…), rendered as a structured card
  // (ConnectorMessageCard) instead of a plain user bubble. Generalizes to any connector via the
  // registry — no per-connector special-casing.
  | { kind: "connector"; source: MessageSource }
  // `persona`: who answered, when the person addressed another assistant with `@id`.
  | { kind: "assistant"; text: string; ts?: number; reasoning?: string; persona?: string }
  // Sub-threads this turn fanned out into. Rendered under the reply as links, so the
  // threads are reachable from the conversation that created them and not only from
  // the sidebar.
  | { kind: "threads"; threads: { session_id: string; title: string }[] }
  // `hidden` = results the user's privacy filters removed before the agent saw them
  // (from the tool message's `_display` sidecar; the agent-visible content has no trace).
  // `standingRule` = the task-scoped rule that auto-allowed this call ("tool → target").
  | { kind: "tool"; id: string; name: string; args: any; status: string; preview?: string; hidden?: number; standingRule?: string }
  | {
      kind: "approval";
      name: string;
      args: any;
      reason: string;
      category?: string;
      // The exact target a standing rule could pin (server-computed) — with a run
      // context, the card offers "Allow every time" (§25).
      standingTarget?: string;
      resolved?: ApprovalDecision;
    }
  | {
      kind: "dirreq";
      reason: string;
      path?: string;
      writable?: boolean;
      resolved?: "granted" | "denied";
    }
  | {
      kind: "planreq";
      plan: string;
      resolved?: "approved" | "rejected";
    }
  | {
      // A live ask_user prompt — the agent's questions, asked one at a time, each with
      // the answers it thinks are likely. Attended sessions answer inline; unattended
      // ones route to the Inbox.
      kind: "question";
      questions: { question: string; options: { label: string; description?: string }[]; multi?: boolean }[];
      resolved?: string;
    }
  | {
      // A live `ask_form` prompt — values only the user holds. Its sibling `question`
      // is for a decision and carries options; this one carries fields.
      kind: "form";
      form: { title?: string; intro?: string; fields: { name: string; label: string; kind?: "text" | "number" | "date"; placeholder?: string; help?: string; required?: boolean }[] };
      resolved?: string;
    }
  | { kind: "notice"; tone: "info" | "warn"; text: string; retriable?: boolean }
  // A screen the assistant's browser kept when it opened a page. The frame is what the page
  // looked like THEN — a finished turn never shows today's screen in its place.
  | { kind: "computer"; persona: string; frame: string; url: string; title: string }
  // The assistant is waiting for the user's hands in its browser (a sign-in, a CAPTCHA).
  | { kind: "computer_help"; persona: string; reason: string; url: string; resolved?: "taken" | "declined" | "ended" }
  // The assistant is waiting for one value typed straight into a page field it never sees.
  | { kind: "computer_secret"; persona: string; label: string; resolved?: "sent" | "declined" | "ended" };
