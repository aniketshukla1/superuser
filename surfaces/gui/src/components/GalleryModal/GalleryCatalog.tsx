import type { GalleryPersona } from "../../api";
import { PersonaHero } from "../PersonaHero";
import { CARD, CHIP, ConnectorChip, sourceOf, type Source } from "./CARD";

/** The gallery's list: source filter, featured strip, every persona, the team teaser. */
export function GalleryCatalog({
  cards,
  installed,
  query,
  source,
  setSource,
  unavailable,
  signedIn,
  openDetail,
}: {
  cards: GalleryPersona[];
  installed: Set<string>;
  query: string;
  source: Source;
  setSource: (s: Source) => void;
  unavailable: boolean;
  signedIn: boolean;
  openDetail: (slug: string) => void;
}) {
  const q = query.trim().toLowerCase();
  const visible = cards.filter(
    (p) =>
      (source === "all" || sourceOf(p) === source) &&
      (!q || `${p.name} ${p.tagline} ${p.description}`.toLowerCase().includes(q)),
  );
  const featured = visible.filter((p) => p.featured);
  const teamCount = cards.filter((p) => sourceOf(p) === "team").length;

  return (
    <div data-testid="gallery-cards">
      <div className="flex items-center gap-2 mb-4">
        {(
          [
            ["all", "All"],
            ["superuser", "From Superuser"],
            ["team", "From your team"],
          ] as [Source, string][]
        ).map(([key, label]) => (
          <button
            key={key}
            className={
              "text-xs px-2.5 py-1 rounded-full border " +
              (source === key
                ? "border-accent text-accent bg-accentSoft"
                : "border-line text-muted hover:border-lineStrong")
            }
            onClick={() => setSource(key)}
          >
            {label}
          </button>
        ))}
      </div>

      {unavailable && signedIn && (
        <div className="text-sm text-muted">
          The gallery is unreachable right now — try again in a moment.
        </div>
      )}

      {featured.length > 0 && (
        <>
          <div className="text-xs text-faint font-semibold mb-2">
            Featured
          </div>
          <div className="flex gap-3 overflow-x-auto hairline-scroll pb-2 mb-5" data-testid="gallery-featured">
            {featured.map((p) => (
              <div
                key={p.slug}
                className="w-[240px] shrink-0 rounded-xl border border-line bg-panel/60 overflow-hidden cursor-pointer hover:border-lineStrong"
                onClick={() => openDetail(p.slug)}
              >
                <PersonaHero slug={p.slug} height={88} />
                <div className="p-3">
                  <div className="text-md font-semibold">{p.name}</div>
                  <div className="text-sm text-muted leading-snug mt-0.5 mb-2">{p.tagline}</div>
                  <div className="flex flex-wrap gap-1.5">
                    {p.recommended_connectors.slice(0, 3).map((c) => (
                      <ConnectorChip key={c} name={c} />
                    ))}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      <div className="text-xs text-faint font-semibold mb-2">
        All personas
      </div>
      <div className="space-y-2">
        {visible.map((p) => {
          const isInstalled = installed.has(p.slug);
          return (
            <div
              className={CARD + " p-3.5 flex items-center gap-4 cursor-pointer hover:border-lineStrong"}
              key={p.slug}
              data-testid={`gallery-${p.slug}`}
              onClick={() => openDetail(p.slug)}
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 mb-0.5">
                  <span className="font-semibold text-md">{p.name}</span>
                  <span className={CHIP}>{p.family}</span>
                  <span className="text-xs text-faint">
                    v{p.version} · {p.publisher}
                  </span>
                </div>
                <div className="text-sm text-muted mb-1.5">{p.tagline}</div>
                {p.recommended_connectors.length > 0 && (
                  <div className="flex flex-wrap gap-1.5">
                    {p.recommended_connectors.map((c) => (
                      <ConnectorChip key={c} name={c} />
                    ))}
                  </div>
                )}
              </div>
              <div className="shrink-0 flex items-center">
                {isInstalled ? (
                  <span className="text-sm text-muted">Installed</span>
                ) : (
                  <span className="text-sm text-accent">View & install →</span>
                )}
              </div>
            </div>
          );
        })}
        {visible.length === 0 && !unavailable && (
          <div className="text-sm text-muted py-4">
            {source === "team"
              ? "Nothing shared with your team yet."
              : q
              ? "No personas match your search."
              : "No personas published yet."}
          </div>
        )}
      </div>

      {source !== "team" && teamCount === 0 && (
        <div className="mt-5 pt-3 border-t border-line text-sm text-faint" data-testid="gallery-team-teaser">
          From your team — nothing shared yet. Publishing a persona to your teammates is coming soon.
        </div>
      )}
    </div>
  );
}
