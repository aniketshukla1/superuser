import { useState } from "react";
import { Icon, ICON } from "../Icon";
import { PanelHead } from "../IntegrationsView";
import { GalleryModal } from "../GalleryModal";
import { PersonasTab } from "../PersonasTab";

// -- Personas: installed/enabled/delete management, the dir/Git importer, and the
// entry point to the Persona Gallery (a screen-sized modal — installs finish back
// here, disabled pending consent; a gallery install re-mounts the list in place).
export function PersonasSection({ onOpenPersona }: { onOpenPersona?: (id: string) => void }) {
  const [galleryBump, setGalleryBump] = useState(0);
  const [galleryOpen, setGalleryOpen] = useState(false);

  return (
    <section>
      <PanelHead
        title="Personas"
        sub="Which assistants are enabled and shown in the picker, plus installing new persona bundles."
      />
      <PersonasTab key={galleryBump} onOpenPersona={onOpenPersona} />
      <button
        className="mt-6 w-full rounded-xl2 border border-line bg-panel px-4 py-3.5 flex items-center gap-3 text-left hover:border-lineStrong"
        data-testid="gallery-link"
        onClick={() => setGalleryOpen(true)}
      >
        <Icon name="sparkle" size={ICON.md} className="text-accent shrink-0" />
        <span className="min-w-0 flex-1">
          <span className="block text-md font-medium">Browse the Persona Gallery</span>
          <span className="block text-sm text-muted">
            Curated assistants from the Superuser team — see what each can do before installing.
          </span>
        </span>
        <span className="text-sm text-accent shrink-0">Open →</span>
      </button>
      {galleryOpen && (
        <GalleryModal
          onClose={() => setGalleryOpen(false)}
          onInstalled={() => setGalleryBump((b) => b + 1)}
        />
      )}
    </section>
  );
}
