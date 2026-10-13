import { useEffect, useState } from "react";
import { deletePersona, getPersonas, getSessions, importAgentPackage, installPersona, previewAgentPackage, updatePersona, type PackagePlan, type Persona, type PersonaConsent } from "../../api";
import type { SessionInfo } from "../../types";
import { CARD, SEC_H } from "../../ui";
import { PackagePlanView } from "../PackagePlanView";
import { BTN_ACCENT, INPUT, SELECT } from "./CHECK";
import { ConsentList } from "./ConsentList";
import { PersonaRow } from "./PersonaRow";

export function PersonasTab({ onOpenPersona }: { onOpenPersona?: (id: string) => void }) {
  const [personas, setPersonas] = useState<Persona[]>([]);
  const [mode, setMode] = useState<"git" | "dir" | "package">("git");
  // An agent package is read into a plan first; nothing is written until Import.
  const [plan, setPlan] = useState<{ dir: string; plan: PackagePlan } | null>(null);
  const [src, setSrc] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [consent, setConsent] = useState<PersonaConsent[] | null>(null);
  const [confirmDel, setConfirmDel] = useState<string | null>(null);
  // Disabling archives the persona's conversations (server-side), so when there are any we
  // arm an inline confirm (same two-step idiom as delete) instead of flipping immediately.
  const [confirmOff, setConfirmOff] = useState<string | null>(null);
  const [sessions, setSessions] = useState<SessionInfo[]>([]);

  const reload = () => getPersonas().then(setPersonas).catch(() => {});
  const reloadSessions = () => getSessions().then(setSessions).catch(() => {});
  useEffect(() => {
    reload();
    reloadSessions();
  }, []);

  // Real conversations the disable would archive (unarchived; run sessions are server-hidden).
  const liveCount = (id: string) =>
    sessions.filter((s) => s.agent === id && !s.archived).length;

  const toggle = async (
    id: string,
    body: { enabled?: boolean; surfaced?: boolean; default?: boolean },
  ) => {
    const r = await updatePersona(id, body);
    if (r.personas) setPersonas(r.personas);
    else reload();
    if (body.enabled === false) reloadSessions(); // counts just changed
  };

  const requestDisable = (p: Persona) => {
    if (liveCount(p.id) > 0) setConfirmOff(p.id);
    else toggle(p.id, { enabled: false });
  };

  const remove = async (id: string) => {
    setConfirmDel(null);
    const r = await deletePersona(id);
    if (!r.ok) {
      setMsg(r.error || "delete failed");
      return;
    }
    if (r.personas) setPersonas(r.personas);
    else reload();
  };

  const readPackage = async () => {
    setBusy(true);
    setMsg(null);
    setConsent(null);
    setPlan(null);
    const dir = src.trim();
    const r = await previewAgentPackage(dir);
    setBusy(false);
    if (!r.ok || !r.plan) {
      setMsg(r.error || "Couldn't read that package.");
      return;
    }
    setPlan({ dir, plan: r.plan });
  };

  const importPackage = async () => {
    if (!plan) return;
    setBusy(true);
    const r = await importAgentPackage(plan.dir, plan.plan.digest);
    setBusy(false);
    if (!r.ok) {
      setMsg(r.error || "Import failed.");
      return;
    }
    setPlan(null);
    setConsent(r.consent || []);
    if (r.personas) setPersonas(r.personas);
    const n = (r.consent || []).length;
    const k = (r.skills || []).length;
    setMsg(`Imported ${n} assistant${n === 1 ? "" : "s"} and ${k} skill${k === 1 ? "" : "s"} — the assistants are off until you enable them above.`);
    setSrc("");
  };

  const install = async () => {
    if (!src.trim()) return;
    if (mode === "package") return readPackage();
    setBusy(true);
    setMsg(null);
    setConsent(null);
    const r = await installPersona(
      mode === "git" ? { git_url: src.trim() } : { dir: src.trim() },
    );
    setBusy(false);
    if (!r.ok) {
      setMsg(r.error || "install failed");
      return;
    }
    setConsent(r.consent || []);
    if (r.personas) setPersonas(r.personas);
    setMsg(`Installed ${(r.consent || []).length} persona(s) — review and enable below.`);
    setSrc("");
  };

  return (
    <div>
      <p className="text-sm text-muted mb-3 leading-prose">
        Enable an assistant, then choose whether it appears in the new-session picker. The starred persona
        is the default for new sessions.
      </p>

      <div className={CARD + " divide-y divide-line mb-6"}>
        {personas.map((p) => (
          <PersonaRow
            key={p.id}
            p={p}
            liveCount={liveCount(p.id)}
            confirmDel={confirmDel === p.id}
            setConfirmDel={(on) => setConfirmDel(on ? p.id : null)}
            confirmOff={confirmOff === p.id}
            setConfirmOff={(on) => setConfirmOff(on ? p.id : null)}
            toggle={(body) => toggle(p.id, body)}
            requestDisable={() => requestDisable(p)}
            remove={() => remove(p.id)}
            onOpenPersona={onOpenPersona}
          />
        ))}
      </div>

      <div className={SEC_H + " mb-1.5"}>Add personas</div>
      <p className="text-sm text-muted mb-3 leading-prose">
        Load from a local directory or a public GitHub repo. Files are copied into a managed area (a
        snapshot), so the persona stays stable even if the source changes. No code runs — a persona only
        composes vetted tools.
      </p>
      <div className="flex items-center gap-2">
        <select
          className={SELECT}
          value={mode}
          onChange={(e) => {
            setMode(e.target.value as "git" | "dir" | "package");
            setPlan(null);
          }}
          data-testid="install-mode"
        >
          <option value="git">GitHub URL</option>
          <option value="dir">Local directory</option>
          <option value="package">Agent package (folder)</option>
        </select>
        <input
          className={INPUT}
          placeholder={
            mode === "git" ? "https://github.com/acme/ops-persona" : mode === "package" ? "/path/to/package" : "/path/to/personas"
          }
          data-testid="install-source"
          value={src}
          onChange={(e) => setSrc(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && install()}
        />
        <button className={BTN_ACCENT} disabled={busy || !src.trim()} onClick={install} data-testid="install-go">
          {mode === "package" ? (busy && !plan ? "Reading…" : "Read package") : busy ? "Installing…" : "Install"}
        </button>
      </div>
      {msg && <div className="text-sm text-muted mt-2.5">{msg}</div>}
      {plan && (
        <PackagePlanView plan={plan.plan} busy={busy} onImport={() => void importPackage()} onCancel={() => setPlan(null)} />
      )}

      {consent && <ConsentList consent={consent} />}
    </div>
  );
}
