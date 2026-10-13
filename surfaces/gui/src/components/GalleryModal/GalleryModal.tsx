import { useEffect, useState } from "react";
import { cloudLogin, getCloudGallery, getCloudGalleryDetail, getCloudStatus, getPersonas, installPersona, type CloudStatus, type GalleryDetail, type GalleryPersona } from "../../api";
import { Icon, ICON } from "../Icon";
import { BTN_ACCENT, CARD, type Source } from "./CARD";
import { GalleryCatalog } from "./GalleryCatalog";
import { GalleryDetailView } from "./GalleryDetailView";

export function GalleryModal({
  onClose,
  onInstalled,
}: {
  onClose: () => void;
  onInstalled?: () => void;
}) {
  const [cloud, setCloud] = useState<CloudStatus | null>(null);
  const [cards, setCards] = useState<GalleryPersona[]>([]);
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [unavailable, setUnavailable] = useState(false);
  const [busy, setBusy] = useState(false);
  const [signingIn, setSigningIn] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [source, setSource] = useState<Source>("all");
  const [detailSlug, setDetailSlug] = useState<string | null>(null);
  const [detail, setDetail] = useState<GalleryDetail | null>(null);
  const [justInstalled, setJustInstalled] = useState(false);

  const reload = async () => {
    setLoading(true);
    const status = getCloudStatus().then(setCloud).catch(() => setCloud(null));
    getPersonas()
      .then((ps) => setInstalled(new Set(ps.map((p) => p.id))))
      .catch(() => {});
    try {
      const g = await getCloudGallery();
      setCards(g.ok ? g.personas : []);
      setUnavailable(!g.ok);
    } catch {
      setCards([]);
      setUnavailable(true);
    }
    // The signed-in check gates which body renders — wait for it too, so the
    // skeleton never flashes into the wrong state.
    await status;
    setLoading(false);
  };
  useEffect(() => {
    reload();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const signIn = async () => {
    setSigningIn(true);
    await cloudLogin(); // sidecar opens the browser; poll for completion
    setTimeout(() => {
      setSigningIn(false);
      reload();
    }, 3000);
  };

  const openDetail = async (slug: string) => {
    setDetailSlug(slug);
    setDetail(null);
    setMsg(null);
    setJustInstalled(false);
    const d = await getCloudGalleryDetail(slug).catch(() => null);
    setDetail(d ?? { ok: false, error: "could not load details" });
  };

  const install = async (slug: string) => {
    setBusy(true);
    setMsg(null);
    const r = await installPersona({ gallery_slug: slug });
    setBusy(false);
    if (!r.ok) {
      setMsg(r.error || "install failed");
      return;
    }
    setInstalled((s) => new Set(s).add(slug));
    setJustInstalled(true);
    onInstalled?.(); // re-mounts the Personas list so the new persona shows in place
  };

  return (
    <div className="fixed inset-0 z-50" data-testid="gallery-modal">
      <div className="absolute inset-0 bg-black/30 backdrop-blur-[1px]" onClick={onClose} />
      <div className="absolute left-1/2 top-[6vh] -translate-x-1/2 w-[720px] max-w-[94vw] max-h-[88vh] rounded-xl2 border border-line bg-panel shadow-2xl overflow-hidden flex flex-col">
        <div className="px-5 pt-4 pb-3 border-b border-line flex items-center gap-3 shrink-0">
          <div className="min-w-0 flex-1">
            <div className="text-lg font-semibold">Persona Gallery</div>
            <div className="text-sm text-muted">
              Curated assistants · installs stay disabled until you approve them
            </div>
          </div>
          {cloud?.signed_in && !detailSlug && (
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search personas"
              className="w-[180px] px-3 py-1.5 rounded-lg border border-line bg-paper text-sm text-ink outline-none focus:border-accent"
            />
          )}
          <button
            className="text-faint hover:text-ink shrink-0"
            onClick={onClose}
            aria-label="Close gallery"
            data-testid="gallery-close"
          >
            <Icon name="x" size={ICON.md} />
          </button>
        </div>

        <div className="overflow-y-auto hairline-scroll p-5">
          {loading ? (
            <div className="space-y-2" data-testid="gallery-loading" aria-busy="true">
              <div className="text-sm text-muted mb-3">Loading the gallery…</div>
              {[0, 1, 2].map((i) => (
                <div key={i} className={CARD + " p-3.5 animate-pulse"}>
                  <div className="h-3.5 w-44 rounded bg-line mb-2.5" />
                  <div className="h-3 w-72 max-w-full rounded bg-line/60" />
                </div>
              ))}
            </div>
          ) : cloud && !cloud.signed_in ? (
            <div className={CARD + " p-5 flex items-center gap-4"} data-testid="gallery-signin">
              <div className="min-w-0 flex-1">
                <div className="font-semibold text-lg mb-1">Sign in to browse the Gallery</div>
                <div className="text-sm text-muted leading-prose">
                  The Gallery is a curated set of assistants from Superuser Cloud and needs a
                  (free) cloud sign-in. Installing personas from a folder or Git URL — on the
                  Personas page — always works without an account.
                </div>
              </div>
              <button className={BTN_ACCENT} onClick={signIn} disabled={signingIn}>
                {signingIn ? "Check your browser…" : "Sign in"}
              </button>
            </div>
          ) : detailSlug ? (
            <GalleryDetailView
              slug={detailSlug}
              detail={detail}
              installed={installed.has(detailSlug)}
              busy={busy}
              msg={msg}
              justInstalled={justInstalled}
              onInstall={() => install(detailSlug)}
              onBack={() => setDetailSlug(null)}
              onClose={onClose}
            />
          ) : (
            <GalleryCatalog
              cards={cards}
              installed={installed}
              query={query}
              source={source}
              setSource={setSource}
              unavailable={unavailable}
              signedIn={!!cloud?.signed_in}
              openDetail={openDetail}
            />
          )}
        </div>
      </div>
    </div>
  );
}
