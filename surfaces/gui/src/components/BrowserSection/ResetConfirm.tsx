import { resetComputer, type ComputerStatus } from "../../api";
import { BTN_BORDERED } from "./BTN_ACCENT";

type Run = (key: string, work: () => Promise<{ ok: boolean; error?: string } | null>, failure: string) => Promise<boolean>;

/** "Sign out of everything": deletes the assistant's browser profile, so it asks first. */
export function ResetConfirm({ c, busy, run, onClose }: { c: ComputerStatus; busy: string; run: Run; onClose: () => void }) {
  return (
    <div className="fixed inset-0 z-50" role="dialog" aria-modal="true" aria-label="Sign out of everything" data-testid="browser-reset-confirm">
      <div className="absolute inset-0 bg-black/30" onClick={onClose} />
      <div className="absolute left-1/2 top-[22vh] -translate-x-1/2 w-[440px] max-w-[92vw] rounded-xl2 border border-line bg-panel shadow-2xl p-4">
        <h2 className="text-lg font-semibold text-ink">Sign {c.name || c.persona} out of everything?</h2>
        <p className="text-sm text-ink mt-2 leading-prose">
          This deletes its browser profile — every site it is signed in to, its cookies and saved state — and every
          screen kept from its past conversations. It can&rsquo;t be undone. Its conversations and files stay.
        </p>
        <div className="flex items-center gap-2 mt-4">
          <span className="flex-1" />
          <button autoFocus className={BTN_BORDERED} onClick={onClose}>
            Cancel
          </button>
          <button
            className="ctl bg-danger text-onAccent shrink-0 disabled:opacity-40"
            disabled={busy === "reset"}
            onClick={async () => {
                        await run("reset", () => resetComputer(c.persona), `Couldn't sign ${c.name || c.persona} out.`);
              onClose();
            }}
            data-testid="browser-reset-go"
          >
            Sign out of everything
          </button>
        </div>
      </div>
    </div>
  );
}
