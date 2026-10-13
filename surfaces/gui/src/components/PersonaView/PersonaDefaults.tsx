import type { PersonaDetail } from "../../api";
import { MODES } from "../CreateAssistantModal";

/** The defaults footer: model needs, the default mode (a control), and its workspace. */
export function PersonaDefaults({
  detail,
  saveMode,
}: {
  detail: PersonaDetail;
  saveMode: (mode: string) => void;
}) {
  return (
    <>
      {/* defaults footer */}
      <section className="flex flex-wrap gap-x-8 gap-y-2 text-sm">
        {/* Replaces a list of model ids nothing declared, so this row never
            rendered at all. This says something a reader can act on: it is about
            the work, not about a model they may not have. */}
        {detail.needs_capable_model && (
          <div>
            <span className="text-faint">Model</span> · needs a capable one
          </div>
        )}
        {detail.default_permission_mode && (
          <div className="flex items-center gap-2">
            <span className="text-faint">Default mode</span>
            {/* Was static text on a page titled "Configure". This is the most
                consequential per-assistant setting there is — it decides how much
                happens without you — so it is the one thing that most needed to be
                a control rather than a label. */}
            <select
              className="bg-panel border border-line rounded-sm px-2 py-1 text-sm text-ink"
              data-testid="persona-mode"
              value={detail.default_permission_mode}
              onChange={(e) => void saveMode(e.target.value)}
            >
              {MODES.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.label}
                </option>
              ))}
            </select>
            <span className="text-xs text-faint">
              {MODES.find((m) => m.id === detail.default_permission_mode)?.hint}
            </span>
          </div>
        )}
        {detail.workspace && (
          <div>
            <span className="text-faint">Workspace</span> · {detail.workspace}
          </div>
        )}
      </section>
    </>
  );
}
