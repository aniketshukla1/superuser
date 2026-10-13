import type { Facet } from "../../api";
import { CUE, evidenceLine, fieldOf, relative } from "./CLASSES";

/**
 * One belief, with the words behind it. The evidence line is the point of the row:
 * "Wants short answers" is an assertion, and the sentence that produced it is what
 * lets someone judge whether it's right.
 */
export function FacetRow({
  facet,
  busy,
  onPin,
  onForget,
}: {
  facet: Facet;
  busy: boolean;
  onPin: () => void;
  onForget: () => void;
}) {
  return (
    <div className="py-2 border-b border-line/60" data-testid="profile-facet">
      <div className="flex items-center gap-2">
        <span className="text-md text-ink">{facet.value}</span>
        {facet.pinned && <span className="text-xs text-accent shrink-0">pinned</span>}
        <span className="text-xs text-faint ml-auto shrink-0">
          {/* A form field says which field; everything else says how it was learned. The
              row used to say both "from your profile" and, underneath, "Your profile ▸ Name". */}
          {facet.from_form && facet.evidence ? `from your profile · ${fieldOf(facet.evidence)}` : CUE[facet.cue]}
          {facet.observations > 1 ? ` · ${facet.observations}×` : ""}
          {/* "when" is only meaningful for something that was observed. A form
              field is regenerated on every read, so its timestamp is always
              "just now" and says nothing about the field. */}
          {facet.from_form ? "" : ` · ${relative(facet.last_seen)}`}
        </span>
      </div>
      {evidenceLine(facet) && <div className="text-sm text-faint mt-0.5 truncate">“{evidenceLine(facet)}”</div>}
      {/* A form field is edited in the form. Pinning something that never decays
          does nothing, and forgetting it would leave the field sitting on screen
          not applying — so the evidence line above, which names the field, is the
          only control it needs. */}
      {!facet.from_form && (
        <div className="jrn-actions" style={{ marginTop: 4 }}>
          <button className="jrn-ghost" disabled={busy} onClick={onPin}>
            {facet.pinned ? "Unpin" : "Pin"}
          </button>
          <button className="jrn-ghost" disabled={busy} onClick={onForget}>
            Forget
          </button>
        </div>
      )}
    </div>
  );
}
