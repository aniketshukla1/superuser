import { CHIP, KIND_TABS } from "./ICON_FOR";

/** Kind and assistant chips; each row appears only once there are two things to tell apart. */
export function InboxFilters({
  kindsPresent,
  personasWithItems,
  kind,
  setKind,
  personaFilter,
  setPersonaFilter,
}: {
  kindsPresent: number;
  personasWithItems: { id: string; label: string }[];
  kind: string;
  setKind: (k: string) => void;
  personaFilter: string;
  setPersonaFilter: (p: string) => void;
}) {
  if (!(kindsPresent > 1 || personasWithItems.length > 1)) return null;
  return (
    <div className="flex items-center gap-2 flex-wrap mb-4" data-testid="inbox-filters">
      {kindsPresent > 1 &&
        KIND_TABS.map((t) => (
          <button
            key={t.key}
            className={CHIP(kind === t.key)}
            aria-pressed={kind === t.key}
            onClick={() => setKind(t.key)}
          >
            {t.label}
          </button>
        ))}
      {personasWithItems.length > 1 && (
        <>
          {kindsPresent > 1 && <span className="w-px h-4 bg-line mx-1" aria-hidden />}
          <button
            className={CHIP(personaFilter === "all")}
            aria-pressed={personaFilter === "all"}
            onClick={() => setPersonaFilter("all")}
          >
            All assistants
          </button>
          {personasWithItems.map((p) => (
            <button
              key={p.id}
              className={CHIP(personaFilter === p.id)}
              aria-pressed={personaFilter === p.id}
              onClick={() => setPersonaFilter(p.id)}
            >
              {p.label}
            </button>
          ))}
        </>
      )}
    </div>
  );
}
