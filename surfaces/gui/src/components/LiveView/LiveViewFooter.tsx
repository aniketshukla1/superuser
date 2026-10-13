import { BTN_ACCENT, BTN_BORDERED } from "./BTN_ACCENT";

/** The strip under the screen: the close-while-driving question, a notice, or the hint. */
export function LiveViewFooter({
  who,
  closing,
  notice,
  busy,
  onRelease,
  onClose,
}: {
  who: string;
  closing: boolean;
  notice: string;
  busy: boolean;
  onRelease: () => Promise<boolean>;
  onClose: () => void;
}) {
  return (
    <div className="flex items-center gap-2 px-4 py-2.5 border-t border-line text-sm">
      {closing ? (
        <>
          <span className="flex-1 text-ink">You still have control. Hand the browser back to {who}?</span>
          <button
            className={BTN_ACCENT}
            disabled={busy}
            onClick={async () => {
              if (await onRelease()) onClose();
            }}
            data-testid="live-view-close-release"
          >
            Hand back and close
          </button>
          <button className={BTN_BORDERED} onClick={onClose} data-testid="live-view-close-keep">
            Keep control and close
          </button>
        </>
      ) : notice ? (
        <span className="flex-1 text-muted">{notice}</span>
      ) : (
        <span className="flex-1 text-muted">Watching. Take control to click and type in it yourself.</span>
      )}
    </div>
  );
}
