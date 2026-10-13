import type { PersonaDetail } from "../../api";
import { fullPersonaName, shortPersonaName } from "../../personaScope";
import { ICON, AVATAR } from "../Icon";
import { PersonaGlyph } from "../personaIcon";
import { Toggle } from "../Toggle";
import { SEC_H } from "../../ui";

/** Who it is: identity and the enable switch, what it is about, its tools and its skills. */
export function PersonaSummary({
  detail,
  personaId,
  toggleEnabled,
}: {
  detail: PersonaDetail;
  personaId: string;
  toggleEnabled: (next: boolean) => void;
}) {
  return (
    <>
      {/* identity + enable */}
      <header className="flex items-start gap-3.5">
        {/* Sized off AVATAR rather than a Tailwind step, so the tile tracks the
            scale it belongs to. It was 48px — taller than the two lines of text
            beside it — chosen when the title was 19px. `family` is passed because
            without it every persona whose manifest names an icon we cannot resolve
            falls through to the generic sparkle instead of its family glyph. */}
        <span
          className="rounded-xl2 bg-panel border border-line grid place-items-center shrink-0"
          style={{ width: AVATAR.lg, height: AVATAR.lg }}
        >
          <PersonaGlyph icon={detail.icon} family={detail.family} size={ICON.lg} />
        </span>
        <div className="min-w-0">
          {/* text-lg, not text-xl. One step down the scale: still above the 13px body
              so the hierarchy holds, without the title shouting over a page whose
              content is mostly small. */}
          <h1 className="text-lg font-semibold tracking-tight text-ink">
            {fullPersonaName(detail.name, personaId)}
          </h1>
          <p className="text-md text-muted mt-0.5">{detail.tagline}</p>
        </div>
        <div className="ml-auto flex items-center gap-2">
          <span className="text-sm text-muted">{detail.enabled ? "Enabled" : "Disabled"}</span>
          <Toggle checked={detail.enabled} onChange={toggleEnabled} title="Enable this persona" />
        </div>
      </header>

      {/* about */}
      {detail.description && (
        <section>
          <div className={`${SEC_H} mb-1.5`}>About</div>
          {/* Prose, so it takes the app's prose treatment: --t-md with --t-prose-lh, the
              same pairing styles.css gives a chat bubble and for the reason stated
              there. It was text-lg, which made the description the largest thing on
              the page after the title — bigger than the tagline it elaborates. */}
          <p className="text-md leading-prose text-ink">{detail.description}</p>
        </section>
      )}

      {/* tools */}
      {detail.tools.length > 0 && (
        <section>
          <div className={`${SEC_H} mb-2`}>Built-in capabilities</div>
          <div className="flex flex-wrap gap-1.5">
            {detail.tools.map((t) => (
              <span
                className="px-2 py-1 rounded-md bg-panel border border-line text-sm font-mono"
                key={t}
              >
                {t}
              </span>
            ))}
          </div>
        </section>
      )}

      {/* skills — the procedures this assistant was given.
          The server has sent `skills` since rosters existed; the client type did
          not declare it, so the page showed what an assistant CAN DO and not what
          it KNOWS HOW TO DO. Same silent contract gap as the missing routes, in
          the other direction: the data was there and nothing read it. */}
      <section>
        <div className={`${SEC_H} mb-1`}>Skills</div>
        {detail.skills && detail.skills.length > 0 ? (
          <>
            <p className="text-sm text-muted mb-2.5">
              The procedures {shortPersonaName(detail.name, personaId)} offers. Type{" "}
              <span className="font-mono">/</span> in the composer to run one.
            </p>
            <div className="flex flex-wrap gap-1.5">
              {detail.skills.map((sk) => (
                <span
                  className="px-2 py-1 rounded-md bg-panel border border-line text-sm font-mono"
                  key={sk}
                >
                  {sk}
                </span>
              ))}
            </div>
          </>
        ) : (
          <p className="text-sm text-muted">
            No named list — {shortPersonaName(detail.name, personaId)} sees every skill
            you have installed.
          </p>
        )}
      </section>
    </>
  );
}
