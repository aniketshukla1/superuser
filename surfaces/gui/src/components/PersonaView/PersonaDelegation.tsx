import type { Persona, PersonaDetail, WorkerType } from "../../api";
import { shortPersonaName } from "../../personaScope";
import { SEC_H } from "../../ui";

/** What it may use beyond itself: workers, its own browser, and other assistants. */
export function PersonaDelegation({
  detail,
  personaId,
  workers,
  others,
  browserError,
  toggleWorker,
  toggleBrowser,
  toggleAsk,
}: {
  detail: PersonaDetail;
  personaId: string;
  workers: WorkerType[];
  others: Persona[];
  browserError: string | null;
  toggleWorker: (name: string, on: boolean) => void;
  toggleBrowser: (on: boolean) => void;
  toggleAsk: (id: string, on: boolean) => void;
}) {
  return (
    <>
      {/* The team. Only for assistants that can delegate — for anything else a
          roster is a choice with no effect, and offering it would suggest
          otherwise. Empty means "whatever is installed", which is the default
          and the thing unticking everything returns you to. */}
      {detail.tools.includes("subagent") && workers.length > 0 && (
        <section data-testid="persona-team">
          <div className={`${SEC_H} mb-2`}>Team</div>
          <p className="text-sm text-muted mb-2">
            Which workers {shortPersonaName(detail.name, personaId)} hands items to. With none
            ticked it uses whatever is installed.
          </p>
          <div className="flex flex-col gap-1.5">
            {workers.map((w) => {
              const on = (detail.subagents ?? []).includes(w.name);
              return (
                <label key={w.name} className="flex items-start gap-2.5">
                  <input
                    type="checkbox"
                    className="mt-0.5"
                    data-testid={`worker-${w.name}`}
                    checked={on}
                    onChange={(e) => void toggleWorker(w.name, e.target.checked)}
                  />
                  <span>
                    <span className="block text-md text-ink font-mono">{w.name}</span>
                    <span className="block text-sm text-muted">{w.description}</span>
                  </span>
                </label>
              );
            })}
          </div>
        </section>
      )}

      {/* Its own browser. Not offered to an assistant whose turns run on a remote agent:
          what that agent can open is its own business, and a grant here would reach nothing. */}
      {!detail.remote && (
        <section data-testid="persona-browser">
          <div className={`${SEC_H} mb-2`}>Its own browser</div>
          <label className="flex items-start gap-2.5">
            <input
              type="checkbox"
              className="mt-0.5"
              data-testid="persona-browser-toggle"
              checked={detail.tools.includes("browser")}
              onChange={(e) => void toggleBrowser(e.target.checked)}
            />
            <span>
              <span className="block text-md text-ink">
                Give {shortPersonaName(detail.name, personaId)} a browser of its own
              </span>
              <span className="block text-sm text-muted">
                It can open pages, and asks you before it clicks or types on each site. Whether the
                browser is switched on at all is in Settings ▸ Browser.
              </span>
            </span>
          </label>
          {browserError && (
            <p role="alert" className="text-sm text-danger mt-1.5" data-testid="persona-browser-error">
              {browserError}
            </p>
          )}
        </section>
      )}

      {/* Who it may ask. A handoff is a run of another assistant with its own tools and
          its own access. Every assistant may ask every other until the person narrows
          it here (owner's call, 2026-10-13; the Email Assistant excepted, it reads
          strangers' mail) — and with nobody ticked the tool is never offered at all. */}
      {others.filter((p) => p.id !== personaId && p.enabled !== false).length > 0 && (
        <section data-testid="persona-asks">
          <div className={`${SEC_H} mb-2`}>Who it may ask</div>
          <p className="text-sm text-muted mb-2">
            {shortPersonaName(detail.name, personaId)} can hand a piece of work to these
            assistants and relay the answer back. Each answers as itself, with its own tools
            and access. Untick any it shouldn't ask; with none ticked it asks nobody.
          </p>
          <div className="flex flex-col gap-1.5">
            {others
              .filter((p) => p.id !== personaId && p.enabled !== false)
              .map((p) => {
                const on = (detail.asks ?? []).includes(p.id);
                return (
                  <label key={p.id} className="flex items-start gap-2.5">
                    <input
                      type="checkbox"
                      className="mt-0.5"
                      data-testid={`asks-${p.id}`}
                      checked={on}
                      onChange={(e) => void toggleAsk(p.id, e.target.checked)}
                    />
                    <span>
                      <span className="block text-md text-ink">{p.name}</span>
                      {p.tagline && <span className="block text-sm text-muted">{p.tagline}</span>}
                    </span>
                  </label>
                );
              })}
          </div>
        </section>
      )}
    </>
  );
}
