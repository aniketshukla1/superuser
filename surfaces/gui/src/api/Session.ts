import { fetch, httpBase, jsonOf, openWebSocket, wsBase } from "./neverRejectingJson";
import type { Handlers } from "./SlackStatus";

export class Session {
  private ws: WebSocket;
  // Payloads sent before the socket finished opening, replayed on `onopen`. Belt-and-suspenders
  // against the first message being dropped if the user sends in the connect window.
  private outbox: object[] = [];

  constructor(sessionId: string, workspace: string, agent: string, handlers: Handlers) {
    const q = `?workspace=${encodeURIComponent(workspace)}&agent=${encodeURIComponent(agent)}`;
    this.ws = openWebSocket(`${wsBase()}/ws/session/${sessionId}${q}`);
    // Guarded like the app-wide events socket above: a truncated frame is a network
    // condition, not a programming error, and an uncaught throw inside a native
    // onmessage callback surfaces as a stackless page error nobody can act on.
    this.ws.onmessage = (e) => {
      let event: unknown;
      try {
        event = JSON.parse(e.data);
      } catch {
        return; /* malformed frame — ignore */
      }
      handlers.onEvent(event as any);
    };
    this.ws.onopen = () => {
      this.flush();
      handlers.onOpen?.();
    };
    this.ws.onclose = () => handlers.onClose?.();
  }

  private flush() {
    if (this.ws.readyState !== WebSocket.OPEN) return;
    const pending = this.outbox;
    this.outbox = [];
    for (const p of pending) this.ws.send(JSON.stringify(p));
  }

  private send(payload: object) {
    if (this.ws.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(payload));
    // Still connecting: queue and flush on open rather than silently dropping.
    else if (this.ws.readyState === WebSocket.CONNECTING) this.outbox.push(payload);
  }

  /** `model` = the composer's CURRENT selection, carried on every message so the turn uses
   * exactly what the user sees — immune to set_model races across reconnects (a new cowork
   * session always reconnects once to adopt its scratch dir, which could drop a queued
   * set_model and leave the engine on a stale/resumed model; found 2026-07-04). */
  userMessage(text: string, attachments?: unknown[], model?: string, skill?: string) {
    this.send({
      type: "user_message",
      text,
      ...(model ? { model } : {}),
      ...(attachments?.length ? { attachments } : {}),
      // Force-run (SKILLS-SPEC §4.1): the composer's /skill pick rides as its own field;
      // the server validates it against the session's effective menu and frames the turn.
      ...(skill ? { skill } : {}),
    });
  }

  approve(decision: string) {
    this.send({ type: "approval", decision });
  }

  // Reply to a `request_directory` prompt: grant a folder (with access level) or decline.
  respondDirectory(granted: boolean, path?: string, writable?: boolean) {
    this.send({ type: "directory_response", granted, ...(path ? { path } : {}), writable: !!writable });
  }

  // Reply to a `propose_plan` prompt: approve (choosing the execution mode) or reject with feedback.
  respondPlan(approved: boolean, mode?: string, feedback?: string) {
    this.send({
      type: "plan_response",
      approved,
      ...(mode ? { mode } : {}),
      ...(feedback ? { feedback } : {}),
    });
  }

  // Answer a live `ask_user` prompt: one entry per question, in order, null where the
  // user skipped. The server hands them to the tool that asked, which pairs them back
  // up with the questions (`Ask::read_answer`).
  respondQuestion(answers: (string | null)[]) {
    this.send({ type: "question_response", answers });
  }

  // Answer a live `ask_form` prompt: one entry per field, keyed by the field's name.
  // The same frame and the same rendezvous as a question — only the shape differs,
  // and the tool that parked the receiver knows which one it asked for
  // (`Form::read_answer`).
  respondForm(answers: Record<string, string>) {
    this.send({ type: "question_response", answers });
  }

  interrupt() {
    this.send({ type: "interrupt" });
  }

  // Re-run a turn that ended in a provider error — no new user message; the server
  // guards on the history tail so a stray frame is a no-op.
  retry() {
    this.send({ type: "retry" });
  }

  setMode(mode: string) {
    this.send({ type: "set_mode", mode });
  }

  setModel(model: string) {
    this.send({ type: "set_model", model });
  }

  close() {
    // Detach before closing: this socket's async `close` event may land AFTER the
    // successor session's `open` (observed when switching into an automation-run
    // session), and a torn-down socket must not clobber the new one's connected state.
    this.ws.onopen = null;
    this.ws.onmessage = null;
    this.ws.onclose = null;
    this.ws.close();
  }
}

export interface FeedbackReason {
  id: string;
  label: string;
}

/** What the user has already said about this session, and the reasons on offer. */
export async function getFeedback(
  session: string,
): Promise<{ reasons: FeedbackReason[]; given: Array<{ message: number; verdict: "up" | "down"; message_at: number }> }> {
  const res = await fetch(`${httpBase()}/v1/feedback?session=${encodeURIComponent(session)}`);
  const j = await jsonOf(res);
  return { reasons: (j.reasons as FeedbackReason[]) ?? [], given: (j.given as any[]) ?? [] };
}

/**
 * Record a verdict on one answer.
 *
 * Keyed on the message's timestamp, not its index in the transcript: several UI items
 * come from one stored message, so an index counted on this side is an index into the
 * wrong list. The server resolves it.
 */
export async function sendFeedback(body: {
  session: string;
  messageAt: number;
  verdict: "up" | "down";
  reason?: string;
  note?: string;
}): Promise<void> {
  await fetch(`${httpBase()}/v1/feedback`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      session: body.session,
      message_at: body.messageAt,
      verdict: body.verdict,
      reason: body.reason,
      note: body.note,
    }),
  });
}
