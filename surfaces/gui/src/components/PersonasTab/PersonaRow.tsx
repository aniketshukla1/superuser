import type { Persona } from "../../api";
import { Icon, ICON } from "../Icon";
import { BTN_BORDERED, CHECK } from "./CHECK";

type Flags = { enabled?: boolean; surfaced?: boolean; default?: boolean };

/** One assistant: enabled, in the picker, default; configure; delete and disable both
 * ask first (disable archives its conversations). */
export function PersonaRow({
  p,
  liveCount,
  confirmDel,
  setConfirmDel,
  confirmOff,
  setConfirmOff,
  toggle,
  requestDisable,
  remove,
  onOpenPersona,
}: {
  p: Persona;
  liveCount: number;
  confirmDel: boolean;
  setConfirmDel: (on: boolean) => void;
  confirmOff: boolean;
  setConfirmOff: (on: boolean) => void;
  toggle: (body: Flags) => void;
  requestDisable: () => void;
  remove: () => void;
  onOpenPersona?: (id: string) => void;
}) {
  return (
    <div className="px-4 py-3">
      <div className="flex items-center gap-4">
      <div className="min-w-0 flex-1">
        <div className="text-md font-medium flex items-center gap-1.5">
          <span className="truncate">{p.name}</span>
          {p.default && <span className="text-accent" title="Default for new sessions">★</span>}
          {p.builtin && <span className="text-xs text-faint font-normal">· built-in</span>}
        </div>
        <div className="text-sm text-muted truncate">{p.tagline}</div>
      </div>
      <label className={CHECK}>
        <input
          type="checkbox"
          checked={p.enabled}
          onChange={(e) =>
            e.target.checked ? toggle({ enabled: true }) : requestDisable()
          }
        />
        Enabled
      </label>
      <label className={CHECK + (p.enabled ? "" : " opacity-40")}>
        <input
          type="checkbox"
          checked={p.surfaced}
          disabled={!p.enabled}
          onChange={(e) => toggle({ surfaced: e.target.checked })}
        />
        In picker
      </label>
      <button
        className={BTN_BORDERED}
        disabled={p.default || !p.enabled}
        onClick={() => toggle({ default: true })}
      >
        Set default
      </button>
      {onOpenPersona && (
        <button
          className="text-faint hover:text-ink shrink-0 p-1"
          title={`Configure ${p.name}`}
          aria-label={`Configure ${p.name}`}
          data-testid={`persona-configure-${p.id}`}
          onClick={() => onOpenPersona(p.id)}
        >
          <Icon name="sliders" size={ICON.md} />
        </button>
      )}
      {!p.builtin &&
        (confirmDel ? (
          <span className="flex items-center gap-1.5 shrink-0">
            <button
              className="text-sm px-2 py-1.5 rounded-lg bg-danger text-white"
              data-testid={`persona-delete-confirm-${p.id}`}
              onClick={remove}
            >
              Delete
            </button>
            <button className={BTN_BORDERED} onClick={() => setConfirmDel(false)}>
              Keep
            </button>
          </span>
        ) : (
          <button
            className="text-faint hover:text-danger shrink-0 p-1"
            title="Delete this persona"
            aria-label={`Delete ${p.name}`}
            data-testid={`persona-delete-${p.id}`}
            onClick={() => setConfirmDel(true)}
          >
            <Icon name="trash" size={ICON.md} />
          </button>
        ))}
      </div>
      {confirmOff && (
        <div
          className="mt-2 flex items-center gap-2.5 text-sm text-muted"
          data-testid={`persona-disable-warning-${p.id}`}
        >
          <span className="min-w-0">
            Disabling archives its {liveCount} conversation
            {liveCount === 1 ? "" : "s"} — they stay available under “Show
            archived”.
          </span>
          <button
            className="text-sm px-2.5 py-1.5 rounded-lg bg-accent text-onAccent shrink-0"
            data-testid={`persona-disable-confirm-${p.id}`}
            onClick={() => {
              setConfirmOff(false);
              toggle({ enabled: false });
            }}
          >
            Disable
          </button>
          <button className={BTN_BORDERED} onClick={() => setConfirmOff(false)}>
            Keep enabled
          </button>
        </div>
      )}
    </div>
  );
}
