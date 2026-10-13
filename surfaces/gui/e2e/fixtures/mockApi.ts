import type { Page } from "@playwright/test";
import { eventSockets } from "./appEvents";
import { connectorState } from "./connectorState";
import { appState } from "./appState";
import { fakeAgent } from "./fakeAgent";
import { sessionRoutes } from "./routes/sessions";
import { personaRoutes } from "./routes/personas";
import { connectorRoutes } from "./routes/connectors";
import { cloudRoutes } from "./routes/cloud";
import { workRoutes } from "./routes/work";
import { miscRoutes } from "./routes/misc";

export type MockState = ReturnType<typeof connectorState> & ReturnType<typeof appState>;
export type Json = (body: unknown, status?: number) => Promise<void>;

// Hermetic API mock. Every /v1 request the GUI makes is fulfilled from the fixtures below (shapes
// mirrored from the real backend), and the event WebSocket is a SCRIPTED FAKE AGENT (ready on
// connect; user_message → turn_start/deltas/assistant_message/turn_done; "run a tool" triggers the
// approval flow), so specs run with no Python server and never touch real state. Mutations
// (sessions, personas, inbox, routing, channel subscriptions) are held in per-test in-memory state
// so add/remove/toggle reflect through the real UI on re-fetch.
export async function mockApi(page: Page) {
  const s: MockState = { ...connectorState(), ...appState() };

  // App-wide event stream: register the socket so sendAppEvent can push into it.
  await page.routeWebSocket(/\/ws\/events$/, (ws) => {
    eventSockets.set(page, ws);
  });

  await page.routeWebSocket(/\/ws\/session\//, fakeAgent);

  await page.route("**/v1/**", async (route) => {
    const req = route.request();
    const p = new URL(req.url()).pathname;
    const m = req.method();
    const json: Json = (body, status = 200) =>
      route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });

    return (
      sessionRoutes(s, req, p, m, json) ??
      personaRoutes(s, req, p, m, json) ??
      connectorRoutes(s, req, p, m, json) ??
      cloudRoutes(s, req, p, m, json) ??
      workRoutes(s, req, p, m, json) ??
      miscRoutes(s, req, p, m, json) ??
      // Anything else: an empty-but-valid body. GET list endpoints read `?? []`/`?? {}` fallbacks.
      json({})
    );
  });
}
