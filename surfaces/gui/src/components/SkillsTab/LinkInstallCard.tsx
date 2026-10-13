import type { SkillLinkPreview } from "../../api";
import { CARD } from "../../ui";
import { BTN_ACCENT, BTN_BORDERED, INPUT } from "./INPUT";

/** Install from a link, in two steps like importing a file: fetch and show the actual
 * instructions, then install exactly what was on screen. */
export function LinkInstallCard({
  linkSource,
  setLinkSource,
  linkPreview,
  setLinkPreview,
  linkBusy,
  fetchLink,
  confirmLink,
}: {
  linkSource: string;
  setLinkSource: (s: string | null) => void;
  linkPreview: SkillLinkPreview | null;
  setLinkPreview: (p: SkillLinkPreview | null) => void;
  linkBusy: boolean;
  fetchLink: () => void;
  confirmLink: () => void;
}) {
  return (
    <div className={`${CARD} p-4 mb-4`}>
      {linkPreview ? (
        <>
          <div className="text-md font-medium mb-1">Install “{linkPreview.name}”?</div>
          <p className="text-xs text-muted mb-3 leading-prose">
            These are instructions your assistant will follow. Read them — whoever wrote this
            gets to steer it, using whatever tools the conversation already has.
          </p>
          <div className="text-sm mb-2">
            <span className="text-muted">{linkPreview.description || "no description"}</span>
          </div>
          {linkPreview.allowed_tools.length > 0 ? (
            <div className="text-xs text-muted mb-2">
              Asks to use: {linkPreview.allowed_tools.join(", ")}
            </div>
          ) : null}
          {linkPreview.replaces_existing ? (
            <div className="text-xs text-warn mb-2">
              Replaces the “{linkPreview.name}” skill you already have.
            </div>
          ) : null}
          <pre className="text-xs font-mono whitespace-pre-wrap max-h-72 overflow-y-auto p-3 rounded-lg bg-paper border border-line">
            {linkPreview.instructions}
          </pre>
          <div className="text-xs text-muted mt-2 break-all">From {linkPreview.url}</div>
          <div className="flex gap-2 mt-3">
            <button className={BTN_ACCENT} disabled={linkBusy} onClick={confirmLink}>
              Install skill
            </button>
            <button className={BTN_BORDERED} onClick={() => setLinkPreview(null)}>
              Back
            </button>
          </div>
        </>
      ) : (
        <>
          <div className="text-md font-medium mb-1">Install from a link</div>
          <p className="text-xs text-muted mb-3 leading-prose">
            A GitHub repo (<code>owner/repo</code>), a link to a skill folder, or a URL
            ending in <code>SKILL.md</code>. Nothing is written until you've read it.
          </p>
          <div className="flex gap-2">
            <input
              className={INPUT}
              autoFocus
              placeholder="owner/repo  or  https://github.com/owner/repo/tree/main/skills/weekly"
              value={linkSource}
              onChange={(e) => setLinkSource(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && fetchLink()}
            />
            <button className={BTN_ACCENT} disabled={linkBusy || !linkSource.trim()} onClick={fetchLink}>
              {linkBusy ? "Fetching…" : "Fetch"}
            </button>
            <button className={BTN_BORDERED} onClick={() => setLinkSource(null)}>
              Cancel
            </button>
          </div>
        </>
      )}
    </div>
  );
}
