import type { Request } from "@playwright/test";
import { AUTOMATION } from "../catalog";
import type { Json, MockState } from "../mockApi";

/** Providers, channels, the inbox and automations. */
export function workRoutes(s: MockState, req: Request, p: string, m: string, json: Json) {
  const { inbox, routing, providers, automations, automationRuns } = s;
  // provider credential check (read-only) — an api_key containing "bad" fails, else ok.
  if (p.endsWith("/v1/providers/verify") && m === "POST") {
    const key = String(req.postDataJSON()?.fields?.api_key || "");
    return /bad/i.test(key)
      ? json({ ok: false, error: "Invalid API key." })
      : json({ ok: true });
  }
  // save a provider key — flips `configured`, stamps key_set_at (backend set_provider parity).
  if (p.endsWith("/v1/providers") && m === "POST") {
    const b = req.postDataJSON();
    const prov = providers.find((x) => x.name === b.name);
    if (!prov) return json({ ok: false, error: `unknown provider: ${b.name}` });
    if (b.fields?.api_key) {
      prov.configured = true;
      prov.key_set_at = "2026-07-05";
    }
    // Backend parity: non-secret fields merge into `values` (empty clears them).
    for (const [k, v] of Object.entries(b.fields || {})) {
      if (k === "api_key") continue;
      if (v) prov.values = { ...prov.values, [k]: v };
      else if (prov.values) delete prov.values[k];
    }
    return json({ ok: true, provider: b.name, recommended_model: null });
  }
  // forget a provider's stored config (Settings ▸ Models "Remove key…").
  if (/\/v1\/providers\/[^/]+$/.test(p) && m === "DELETE") {
    const name = p.split("/").pop()!;
    const prov = providers.find((x) => x.name === name);
    if (!prov) return json({ ok: false, error: `unknown provider: ${name}` });
    prov.configured = !prov.needs_key; // keyless (ollama) stays "configured"
    prov.key_set_at = null;
    return json({ ok: true, provider: name });
  }
  if (p.endsWith("/v1/providers")) return json(providers);
  if (p.endsWith("/v1/channels/recent"))
    return json({
      channels: [
        { channel: "slack:C0AAA111", name: "team-test", last_from: "amy", last_text: "standup at 10" },
        { channel: "slack:C0BBB222", last_from: "bob", last_text: "deploy failed" },
      ],
    });

  // inbox: pending items + the outbound routing binding (inline Slack config)
  if (/\/v1\/inbox\/[^/]+\/resolve$/.test(p) && m === "POST") {
    const id = decodeURIComponent(p.split("/")[p.split("/").length - 2]);
    const it = inbox.find((x) => x.id === id);
    if (it) {
      it.state = "resolved";
      it.resolution = req.postDataJSON().resolution;
    }
    return json({ ok: true });
  }
  if (p.endsWith("/v1/inbox/routing/binding") && m === "POST") {
    const b = req.postDataJSON();
    routing.channel = b.channel;
    routing.target = b.target;
    return json({ ok: true, bindings: [{ ...routing }] });
  }
  if (p.endsWith("/v1/inbox/routing")) return json({ bindings: [{ ...routing }] });
  if (p.endsWith("/v1/inbox")) {
    const q = new URL(req.url()).searchParams;
    const sid = q.get("session_id");
    const state = q.get("state");
    return json({
      items: inbox.filter(
        (i) => (!sid || i.session_id === sid) && (!state || i.state === state),
      ),
    });
  }

  // automations: one scheduled task with a running run (drives the Automations detail page
  // and the run-session banner + Back-to-runs flow). Mutable: Run now appends a run and opens
  // its live session; the enable toggle (PATCH) and delete (DELETE) round-trip through the UI.
  if (/\/v1\/automations\/[^/]+\/seen$/.test(p) && m === "POST") {
    const id = p.split("/").slice(-2)[0];
    const task = automations.find((t) => t.id === id);
    if (task) {
      task.unseen_runs = 0;
      task.unseen_failed = false;
      task.seen_runs_at = Math.floor(Date.now() / 1000);
    }
    return json({ ok: !!task });
  }
  if (/\/v1\/automations\/[^/]+\/run$/.test(p) && m === "POST") {
    const id = p.split("/").slice(-2)[0];
    const task = automations.find((t) => t.id === id);
    if (!task) return json({ ok: false, error: "unknown task" });
    const runId = `r${automationRuns.length + 1}`;
    automationRuns.unshift({
      run_id: runId,
      task_id: id,
      session_id: `__run__${runId}`,
      started_at: Math.floor(Date.now() / 1000),
      finished_at: null,
      status: "running",
      result_text: null,
      artifacts: [],
      error: null,
      trigger: "manual",
    });
    return json({
      ok: true,
      run_id: runId,
      session_id: `__run__${runId}`,
      workspace: task.workspace,
      agent: task.agent,
      prompt: task.instructions,
    });
  }
  if (/\/v1\/automations\/[^/]+$/.test(p) && m === "GET") {
    const id = p.split("/").pop();
    const task = automations.find((t) => t.id === id) ?? automations[0];
    return json({ task, runs: automationRuns.filter((r) => r.task_id === task?.id) });
  }
  if (/\/v1\/automations\/[^/]+$/.test(p) && m === "PATCH") {
    const id = p.split("/").pop();
    const task = automations.find((t) => t.id === id);
    const body = req.postDataJSON() ?? {};
    if (task && body.revoke) {
      // Standing-rule revocation (§25): remove the entry; `revoke` is a command,
      // not a field to Object.assign onto the task.
      task.always_allowed = (task.always_allowed || []).filter(
        (r: any) => r.entry !== body.revoke,
      );
      return json({ ok: true, task });
    }
    if (task) Object.assign(task, body);
    return json({ ok: true, task });
  }
  if (/\/v1\/automations\/[^/]+$/.test(p) && m === "DELETE") {
    const id = p.split("/").pop();
    const i = automations.findIndex((t) => t.id === id);
    if (i >= 0) automations.splice(i, 1);
    return json({ ok: true });
  }
  if (p.endsWith("/v1/automations") && m === "POST") {
    // GUI/onboarding-recipe create (§24) — mirrors the server: title+instructions+cron
    // required; §25 permissions become always_allowed entries (write grants only).
    const body = req.postDataJSON() || {};
    if (!body.title || !body.instructions || !(body.cron || body.fire_at))
      return json({ ok: false, error: "missing fields" });
    const grants = (body.permissions || [])
      .filter((g: any) => g && g.access === "write" && g.tool && g.target)
      .map((g: any) => ({ entry: `${g.tool} ${g.target}`, tool: g.tool, target: g.target }));
    const task = {
      ...AUTOMATION,
      id: `task-ob-${automations.length}`,
      title: body.title,
      instructions: body.instructions,
      schedule: body.cron || body.fire_at,
      always_allowed: grants,
      run_count: 0,
    };
    automations.push(task);
    return json({ ok: true, task });
  }
  if (p.endsWith("/v1/automations")) return json({ tasks: automations });
}
