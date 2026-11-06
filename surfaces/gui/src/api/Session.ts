import { fetch, httpBase, jsonBody, jsonOf, openWebSocket, wsBase } from "./neverRejectingJson";
import type { Handlers } from "./SlackStatus";

/** A frame off the socket, or undefined when it doesn't parse. */
function parseFrame(data: any): unknown {
  try {
    return JSON.parse(data);
  } catch {
    return undefined;
  }
}

/** The live socket for one session: events come in, the user's messages and answers go out. */
export class Session {
  private readonly ws: WebSocket;
  /** Frames sent before the socket finished opening. They go out on `onopen`, so a message
   *  typed while the connection is still being made isn't lost. */
  private queued: object[] = [];

  constructor(sessionId: string, workspace: string, agent: string, handlers: Handlers) {
    const query = [`workspace=${encodeURIComponent(workspace)}`, `agent=${encodeURIComponent(agent)}`].join("&");
    this.ws = openWebSocket(`${wsBase()}/ws/session/${sessionId}?${query}`);
    // A cut-off frame is a network event, not a bug, and a throw inside a native socket
    // callback would show up as a page error with no stack. Such frames are skipped.
    this.ws.onmessage = (message) => {
      const event = parseFrame(message.data);
      if (event !== undefined) handlers.onEvent(event as any);
    };
    this.ws.onopen = () => {
      this.sendQueued();
      handlers.onOpen?.();
    };
    this.ws.onclose = () => handlers.onClose?.();
  }

  private sendQueued() {
    if (this.ws.readyState !== WebSocket.OPEN) return;
    for (const frame of this.queued.splice(0)) this.ws.send(JSON.stringify(frame));
  }

  private deliver(frame: object) {
    const state = this.ws.readyState;
    if (state === WebSocket.OPEN) this.ws.send(JSON.stringify(frame));
    // Still connecting: keep it for `onopen` instead of dropping it.
    else if (state === WebSocket.CONNECTING) this.queued.push(frame);
  }

  /** `model` is what the composer shows right now, sent with every message so the turn runs on
   *  exactly that. It can't be lost to a set_model race across reconnects: a new cowork
   *  session reconnects once to take up its scratch folder, and a queued set_model could be
   *  dropped then, leaving the engine on an old model (found 2026-07-04). */
  userMessage(text: string, attachments?: unknown[], model?: string, skill?: string) {
    const frame: Record<string, unknown> = { type: "user_message", text };
    if (model) frame.model = model;
    if (attachments?.length) frame.attachments = attachments;
    // A skill forced from the composer's "/" menu (SKILLS-SPEC §4.1) travels as its own field;
    // the server checks it against what the session may use and frames the turn around it.
    if (skill) frame.skill = skill;
    this.deliver(frame);
  }

  approve(decision: string) {
    this.deliver({ type: "approval", decision });
  }

  /** The answer to `request_directory`: a folder with its access level, or a refusal. */
  respondDirectory(granted: boolean, path?: string, writable?: boolean) {
    const frame: Record<string, unknown> = { type: "directory_response", granted };
    if (path) frame.path = path;
    frame.writable = Boolean(writable);
    this.deliver(frame);
  }

  /** The answer to `propose_plan`: approved with the mode to run it in, or rejected with feedback. */
  respondPlan(approved: boolean, mode?: string, feedback?: string) {
    const frame: Record<string, unknown> = { type: "plan_response", approved };
    if (mode) frame.mode = mode;
    if (feedback) frame.feedback = feedback;
    this.deliver(frame);
  }

  /** The answers to a live `ask_user`: one per question, in order, null for a skipped one. The
   *  server passes them to the waiting tool, which matches them to its questions
   *  (`Ask::read_answer`). */
  respondQuestion(answers: (string | null)[]) {
    this.deliver({ type: "question_response", answers });
  }

  /** The answers to a live `ask_form`, keyed by field name. Same frame and same hand-off as a
   *  question; only the shape differs, and the waiting tool knows which it asked for
   *  (`Form::read_answer`). */
  respondForm(answers: Record<string, string>) {
    this.deliver({ type: "question_response", answers });
  }

  interrupt() {
    this.deliver({ type: "interrupt" });
  }

  /** Run again a turn that ended on a provider error, with no new message. The server checks
   *  the end of the history, so a stray frame does nothing. */
  retry() {
    this.deliver({ type: "retry" });
  }

  setMode(mode: string) {
    this.deliver({ type: "set_mode", mode });
  }

  setModel(model: string) {
    this.deliver({ type: "set_model", model });
  }

  close() {
    // Unhook first: this socket's close event can arrive after the next session's open
    // (seen when switching into an automation run's session), and a dead socket must not
    // overwrite the new one's connected state.
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

type GivenFeedback = { message: number; verdict: "up" | "down"; message_at: number };

/** What the user has already said about this session, and the reasons they can pick from. */
export async function getFeedback(session: string): Promise<{ reasons: FeedbackReason[]; given: GivenFeedback[] }> {
  const reply = await jsonOf(await fetch(`${httpBase()}/v1/feedback?session=${encodeURIComponent(session)}`));
  return { reasons: (reply.reasons as FeedbackReason[]) ?? [], given: (reply.given as GivenFeedback[]) ?? [] };
}

/** Record a verdict on one answer. It is keyed on the message's timestamp, not its position:
 *  several items on screen come from one stored message, so a position counted here would
 *  point into the wrong list. The server works it out. */
export async function sendFeedback(body: { session: string; messageAt: number; verdict: "up" | "down"; reason?: string; note?: string }): Promise<void> {
  const { session, messageAt, verdict, reason, note } = body;
  await fetch(`${httpBase()}/v1/feedback`, jsonBody("POST", { session, message_at: messageAt, verdict, reason, note }));
}
