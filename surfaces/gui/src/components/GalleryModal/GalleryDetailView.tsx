import type { GalleryDetail } from "../../api";
import { BrandIcon } from "../brandIcons";
import { Markdown } from "../Markdown";
import { PersonaHero } from "../PersonaHero";
import { BTN_ACCENT, CARD, CHIP } from "./CARD";

/** One gallery persona: its card, install, the pitch, and what its manifest says it can do. */
export function GalleryDetailView({
  slug,
  detail,
  installed,
  busy,
  msg,
  justInstalled,
  onInstall,
  onBack,
  onClose,
}: {
  slug: string;
  detail: GalleryDetail | null;
  installed: boolean;
  busy: boolean;
  msg: string | null;
  justInstalled: boolean;
  onInstall: () => void;
  onBack: () => void;
  onClose: () => void;
}) {
  const card = detail?.card;
  const caps = detail?.capabilities;

  return (
    <div data-testid="gallery-detail">
      <button
        className="text-sm text-muted hover:text-ink mb-3"
        onClick={onBack}
      >
        ← Gallery
      </button>
      {!detail ? (
        <div className="text-sm text-muted">Loading…</div>
      ) : !detail.ok || !card ? (
        <div className="text-sm text-danger">{detail.error || "could not load details"}</div>
      ) : (
        <div className="space-y-4">
          <div className="flex items-start gap-4">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 mb-1">
                <span className="font-semibold text-lg">{card.name}</span>
                <span className={CHIP}>{card.family}</span>
              </div>
              <div className="text-md text-muted">{card.tagline}</div>
              <div className="text-xs text-faint mt-1">
                v{card.version} · {card.publisher} · {card.risk_summary}
              </div>
            </div>
            <div className="shrink-0">
              {installed ? (
                <span className="text-sm text-muted">Installed</span>
              ) : (
                <button className={BTN_ACCENT} onClick={onInstall} disabled={busy}>
                  {busy ? "Installing…" : "Install"}
                </button>
              )}
            </div>
          </div>
          {msg && <div className="text-sm text-danger">{msg}</div>}

          {justInstalled && (
            <div className="rounded-lg border border-okLine bg-okSoft px-3.5 py-2.5 flex items-center gap-3">
              <span className="flex-1 text-sm text-ok">
                Installed — it&rsquo;s waiting in Personas, disabled until you approve and enable it.
              </span>
              <button className={BTN_ACCENT} onClick={onClose}>
                Done
              </button>
            </div>
          )}

          <PersonaHero slug={slug} height={128} className="rounded-xl" />

          {card.pitch_markdown && (
            <div className={CARD + " p-4 text-md leading-prose"}>
              <Markdown text={card.pitch_markdown} />
            </div>
          )}

          {caps && (
            <div className={CARD + " p-4"} data-testid="gallery-capabilities">
              <div className="text-md font-semibold mb-2">
                What it can do — verified from its manifest
              </div>
              <div className="text-sm text-faint mb-3">
                Read by this app&rsquo;s own parser, so it matches exactly what the install
                consent will ask you to approve. No executable code is installed.
              </div>
              <div className="space-y-2 text-sm">
                <div>
                  <span className="text-muted">Tools: </span>
                  {caps.tools.join(", ") || "none"}
                  {caps.risk.length > 0 && (
                    <span className="text-faint"> · risk: {caps.risk.join(", ")}</span>
                  )}
                </div>
                <div>
                  <span className="text-muted">Permissions: </span>
                  {caps.recommended_mode} mode
                  {caps.messaging ? " · can use messaging" : ""}
                  {caps.mcp.length > 0 ? ` · MCP: ${caps.mcp.join(", ")}` : ""}
                </div>
                {(detail.recommends?.length ?? 0) > 0 && (
                  <div>
                    <div className="text-muted mb-1.5">Works with these connections:</div>
                    <div className="space-y-1.5">
                      {detail.recommends!.map((r) => (
                        <div key={r.kind + r.ref} className="flex items-baseline gap-2">
                          <span className={CHIP + " inline-flex items-center gap-1 shrink-0"}>
                            <BrandIcon name={r.ref} size={12} />
                            {r.ref}
                            {r.tier === "core" ? " · core" : ""}
                          </span>
                          <span className="text-sm text-faint">{r.reason}</span>
                        </div>
                      ))}
                    </div>
                    <div className="text-xs text-faint mt-2">
                      You connect these yourself (one click when signed in) — installing the
                      assistant grants it nothing until you do.
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
