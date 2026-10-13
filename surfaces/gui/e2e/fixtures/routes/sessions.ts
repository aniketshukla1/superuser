import type { Request } from "@playwright/test";
import { HEALTH, SETTINGS, USAGE_ROLLUP, PINNED_SESSION } from "../data";
import { baseName } from "../catalog";
import type { Json, MockState } from "../mockApi";

/** Session-scoped routes, skills, settings, attachments and workspaces. */
export function sessionRoutes(s: MockState, req: Request, p: string, m: string, json: Json) {
  const { sessions, roots, connections, unattended, skills } = s;

  // session-scoped (id-agnostic — any session resolves to the same fixture).
  // POST = the per-session mute override (§32 Access toggles) — flip the shared state so
  // the section's reload sees the change.
  if (/\/v1\/sessions\/[^/]+\/connections$/.test(p)) {
    if (m === "POST") {
      const b = req.postDataJSON() || {};
      const row = connections.connected.find((c) => c.connector === b.connector);
      // enabled and scope move independently — the real endpoint applies only the
      // field that was actually sent, so muting never resets a chosen scope.
      if (row) {
        if (typeof b.enabled === "boolean") row.enabled = b.enabled;
        if (typeof b.scope === "string") row.scope = b.scope;
      }
      return json({ ok: true });
    }
    return json(connections);
  }
  if (/\/v1\/sessions\/[^/]+\/roots$/.test(p)) {
    if (m === "POST") {
      const b = req.postDataJSON();
      const existing = roots.find((r) => r.path === b.path);
      if (existing) existing.writable = !!b.writable;
      else roots.push({ path: b.path, writable: !!b.writable, label: baseName(b.path), primary: false, exists: true });
      return json({ ok: true, roots });
    }
    if (m === "DELETE") {
      const rp = new URL(req.url()).searchParams.get("path");
      const i = roots.findIndex((r) => r.path === rp && !r.primary);
      if (i >= 0) roots.splice(i, 1);
      return json({ ok: true, roots });
    }
    return json({ roots });
  }
  if (/\/v1\/sessions\/[^/]+\/messages$/.test(p)) return json({ messages: [] });
  if (/\/v1\/sessions\/[^/]+\/unattended$/.test(p)) {
    const id = decodeURIComponent(p.split("/").slice(-2)[0]);
    if (m === "POST") {
      unattended[id] = !!req.postDataJSON().unattended;
      return json({ ok: true, unattended: unattended[id] });
    }
    return json({ unattended: !!unattended[id] });
  }
  if (/\/v1\/sessions\/[^/]+$/.test(p)) {
    const id = decodeURIComponent(p.split("/").pop()!);
    const i = sessions.findIndex((s) => s.session_id === id);
    if (m === "PATCH") {
      const b = req.postDataJSON() || {};
      if (i >= 0) Object.assign(sessions[i], b);
      // A title someone chose makes the session `named` (sessions.rs: a non-empty title);
      // clearing it falls back to the guess from the first message.
      if (i >= 0 && typeof b.title === "string") sessions[i].named = b.title.trim() !== "";
      return json({ ok: true });
    }
    if (m === "DELETE") {
      if (i >= 0) sessions.splice(i, 1);
      return json({ ok: true });
    }
    return json(i >= 0 ? sessions[i] : PINNED_SESSION);
  }

  // Skills (SKILLS-SPEC §5/§6) — the five routes the server has (api_server.rs). Create,
  // upload, reveal and edit were never served, so they are not
  // mocked either: a mock that answers them would pass a spec against a door that is shut.
  if (/\/v1\/sessions\/[^/]+\/skills$/.test(p)) {
    // The composer's live menu: every Settings-enabled skill (§4 — disabled = invisible).
    return json({
      skills: skills
        .filter((s) => s.enabled)
        .map((s) => ({ name: s.name, description: s.description, scope: s.scope, enabled: true })),
    });
  }
  // Install from a link (skills_install.rs): preview fetches and parses and writes NOTHING;
  // install writes exactly the preview the user read. The mock fetches nothing — the skill
  // is named for the source's last path segment, and an https URL that is not a SKILL.md is
  // refused with resolve_source's own sentence.
  if (p.endsWith("/v1/skills/preview") && m === "POST") {
    const source = String((req.postDataJSON() || {}).source || "").trim().replace(/\/+$/, "");
    if (!source)
      return json({ ok: false, error: "give a skill source — `owner/repo`, a GitHub link, or a URL to a SKILL.md" });
    if (source.startsWith("https://") && !source.startsWith("https://github.com/") && !source.endsWith(".md"))
      return json({ ok: false, error: "that URL doesn't point at a SKILL.md file" });
    const name = source.replace(/\/SKILL\.md$/i, "").split("/").pop() || "skill";
    return json({
      ok: true,
      preview: {
        name, description: "says hello", instructions: "Say hello warmly.", allowed_tools: [],
        source,
        url: source.startsWith("https://") ? source : `https://raw.githubusercontent.com/${source}/HEAD/SKILL.md`,
        replaces_existing: skills.some((s) => s.name === name),
      },
    });
  }
  if (p.endsWith("/v1/skills/install") && m === "POST") {
    const pv = (req.postDataJSON() || {}).preview;
    if (!pv?.name) return json({ ok: false, error: "install needs the preview that was approved" });
    // The list reports anything not compiled in as "local" — a link install included.
    const row = {
      name: pv.name, description: pv.description, instructions: pv.instructions,
      scope: "global", source: "local", enabled: true, path: `/state/skills/${pv.name}`, files: 0,
    };
    const i = skills.findIndex((s) => s.name === pv.name);
    if (i >= 0) skills[i] = row;
    else skills.push(row);
    return json({ ok: true, name: pv.name, path: row.path });
  }
  if (/\/v1\/skills\/[^/]+$/.test(p) && (m === "PATCH" || m === "DELETE")) {
    const name = decodeURIComponent(p.split("/").pop()!);
    const i = skills.findIndex((s) => s.name === name);
    if (i < 0) return json({ ok: false, error: `Unknown skill: ${name}` });
    if (m === "DELETE") {
      skills.splice(i, 1);
      return json({ ok: true });
    }
    const b = req.postDataJSON() || {};
    if (typeof b.enabled === "boolean") skills[i].enabled = b.enabled;
    if (typeof b.description === "string") skills[i].description = b.description;
    if (typeof b.instructions === "string") skills[i].instructions = b.instructions;
    return json({ ok: true });
  }
  if (p.endsWith("/v1/skills")) return json({ skills });

  if (p.endsWith("/v1/health")) return json(HEALTH);
  if (p.endsWith("/v1/settings")) return json(SETTINGS);
  // The context ring's panel: tokens this app has spent (usage_rollup.rs). Only read
  // while the panel is open, so no other surface sees it.
  if (p.endsWith("/v1/usage/rollup")) return json(USAGE_ROLLUP);
  if (p.endsWith("/v1/settings/pdf") && m === "POST") {
    Object.assign(SETTINGS, req.postDataJSON());
    return json({
      ok: true,
      pdf_fallback: SETTINGS.pdf_fallback,
      pdf_max_pages: SETTINGS.pdf_max_pages,
      pdf_max_mb: SETTINGS.pdf_max_mb,
    });
  }
  if (p.endsWith("/v1/attachments/inspect-pdf") && m === "POST") {
    // Page count for the composer threshold check: the tests encode it in the PDF body
    // as "%%pages=N" (the mock doesn't parse real PDFs).
    const data = String(req.postDataJSON()?.data_url || "");
    const match = /%%pages=(\d+)/.exec(atob(data.split(",")[1] || "") || "");
    return json({ ok: true, pages: match ? Number(match[1]) : 1, bytes: data.length });
  }
  if (p.endsWith("/v1/workspaces/recent")) return json({ workspaces: [] });
  if (p.endsWith("/v1/workspaces/pick") && m === "POST") {
    return json({ ok: true, path: "/tmp/picked-folder" });
  }
  if (p.endsWith("/v1/workspaces/open") && m === "POST") {
    const b = req.postDataJSON();
    return json({ ok: true, path: b.path, git_branch: "main" });
  }
}
