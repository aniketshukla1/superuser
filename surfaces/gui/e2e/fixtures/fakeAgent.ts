import type { WebSocketRoute } from "@playwright/test";
import { scriptedTurn, type AgentState } from "./agentScripts";
import { PERSONAS } from "./data";

// The scripted fake agent behind the session WebSocket. Speaks the real event protocol
// ({type, data}), so the full send → stream → render loop and the approval round-trip run
// through the production code paths:
//   · on connect: `ready`
//   · user_message: turn_start (with input, exercising the foreground dedupe) → two
//     assistant_deltas → assistant_message "Echo: <text>" → turn_done
//   · a message containing "run a tool": tool_proposed + permission_required, then the turn
//     SUSPENDS until the client's approval decision arrives (deny → skipped; else → ran)
export function fakeAgent(ws: WebSocketRoute) {
  const send = (type: string, data: Record<string, unknown> = {}) =>
    ws.send(JSON.stringify({ type, data }));
  send("ready");
  const st: AgentState = { pendingTool: "run_shell", epicTimer: null, hadTurn: false };
  ws.onMessage((raw) => {
    const msg = JSON.parse(String(raw));
    if (msg.type === "user_message") {
      st.hadTurn = true;
      // Force-run (SKILLS-SPEC §6): like the real server, TURN_START ships the user's
      // literal "/name …" line as `display` so the client dedupes on what the user sees.
      send("turn_start", {
        input: msg.text,
        ...(msg.skill ? { display: `/${msg.skill}${msg.text ? ` ${msg.text}` : ""}` } : {}),
      });
      if (scriptedTurn(msg, send, st)) return;
      send("assistant_delta", { text: "Echo: " });
      send("assistant_delta", { text: msg.text });
      // Echo the model the message carried — pins the model-per-message contract (the
      // composer's visible model must ride on every user_message; 2026-07-04 fix).
      // Same for `skill`: the force-run pick must ride as its OWN FIELD, never as text.
      // `usage` mirrors the real engine's assistant_message sidecar (OPE-42): fixed
      // counts per turn so the usage-chip specs can assert exact accumulation.
      // `@<id> …` opening the message: like the real server (`coworker::mentioned`), that
      // assistant answers, and the reply says so. Only a switched-on fixture persona counts.
      const handle = /^@([\w-]+)/.exec(msg.text)?.[1]?.toLowerCase();
      const persona = PERSONAS.personas.find((p) => p.id === handle && p.enabled)?.id;
      send("assistant_message", {
        ...(persona ? { persona } : {}),
        text: `Echo: ${msg.text} [model=${msg.model || "none"}]${msg.skill ? ` [skill=${msg.skill}]` : ""}`,
        usage: {
          model: msg.model || "anthropic:claude-opus-4-8",
          input: 1_000,
          output: 200,
          cache_read: 8_000,
          cache_write: 800,
        },
      });
      send("turn_done");
    } else if (msg.type === "approval") {
      if (st.pendingTool === "run_shell") {
        if (msg.decision === "deny") {
          send("tool_finished", { name: "run_shell", status: "denied" });
          send("assistant_message", { text: "Understood — skipped the command." });
        } else {
          send("tool_finished", { name: "run_shell", status: "done", result_preview: "README.md" });
          send("assistant_message", { text: "The command ran; 1 file found." });
        }
      } else if (msg.decision === "deny") {
        send("tool_finished", { name: st.pendingTool, status: "denied" });
        send("assistant_message", { text: "Understood — skipped it." });
      } else {
        send("tool_finished", { name: st.pendingTool, status: "done", result_preview: "ok" });
        // The decision echoes back so specs can pin what rode the wire (e.g. always_task).
        send("assistant_message", { text: `Done via ${st.pendingTool} [decision=${msg.decision}]` });
      }
      send("turn_done");
    } else if (msg.type === "interrupt") {
      // Stop mid-stream: like the real engine, end the turn with `interrupted` and
      // NO assistant_message — the client owns promoting the partial into the transcript.
      if (st.epicTimer) {
        clearInterval(st.epicTimer);
        st.epicTimer = null;
      }
      send("interrupted", {});
      send("turn_done");
    } else if (msg.type === "set_model") {
      // Mid-session switch: the server applies it and broadcasts the persisted marker.
      // Like the real server, the FIRST bind (fresh session) is silent.
      if (st.hadTurn)
        send("model_changed", {
          model: msg.model,
          text: `Model switched to ${msg.model}`,
        });
    } else if (msg.type === "retry") {
      // Like the real engine: re-runs with NO new user message (turn_start input is empty).
      send("turn_start", { input: "" });
      send("assistant_message", { text: "Recovered after retry." });
      send("turn_done");
    }
  });
}
