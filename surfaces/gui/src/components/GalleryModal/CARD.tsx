import type { GalleryPersona } from "../../api";
import { BrandIcon } from "../brandIcons";

// The Persona Gallery, as a screen-sized modal over Settings ▸ Personas (the catalog
// wants room the inline section never had; installs finish back on the Personas page,
// which is why this is a modal and not a route). Three zones: header (search + source
// chips), a featured carousel (publisher-flagged), and the catalog list; every card
// opens the in-modal detail page — install only happens there, informed.
//
// Trust model unchanged: browsing requires the (free) cloud sign-in; the pitch is
// publisher metadata but the capabilities card is derived locally from the manifest
// by our own parser; installs land disabled pending consent under Personas.

// Deliberately NOT the shared CARD: a translucent tile inside a modal, where the
// standard opaque panel would read as a second surface stacked on the first.
export const CARD = "rounded-xl border border-line bg-panel/60";
export const BTN_ACCENT =
  "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const CHIP = "ctl rounded border border-line text-muted";

export type Source = "all" | "superuser" | "team";

export function sourceOf(p: GalleryPersona): Exclude<Source, "all"> {
  return p.publisher === "Superuser" ? "superuser" : "team";
}

export function ConnectorChip({ name }: { name: string }) {
  return (
    <span className={CHIP + " inline-flex items-center gap-1"}>
      <BrandIcon name={name} size={12} />
      {name}
    </span>
  );
}
