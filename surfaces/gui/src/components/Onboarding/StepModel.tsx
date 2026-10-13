import { useState } from "react";
import { ProviderCards, ProviderForm, type ProviderSetupState } from "../../providers/ProviderSetup";

/** Step 1: pick a model provider (gallery ⇄ key form, the shared provider machinery). */
export function StepModel({
  ps,
  canNext,
  advance,
  finish,
}: {
  ps: ProviderSetupState;
  canNext: boolean;
  advance: () => void;
  finish: () => void;
}) {
  const [skipConfirm, setSkipConfirm] = useState(false);
  return (
    <section data-testid="ob-step-model" className="flex-1 min-h-0 flex flex-col">
      {/* Persistent header — stays put while the region below swaps (§39). */}
      <h1 className="text-xl font-semibold">Welcome to Superuser<span className="beta-tag">BETA</span></h1>
      <p className="text-md text-muted mt-0.5 mb-4">
        Pick a model provider to get started — Superuser runs on your own key, and your
        key and your data stay on this computer.
      </p>

      {!ps.sel ? (
        /* ---- the provider GALLERY ---- */
        <div className="flex-1 min-h-0 overflow-y-auto pr-1" data-testid="ob-provider-gallery">
          <ProviderCards ps={ps} tp="ob" />
        </div>
      ) : (
        /* ---- one provider's key form, same box ---- */
        <div className="flex-1 min-h-0 overflow-y-auto pr-1">
          <ProviderForm ps={ps} tp="ob" />
        </div>
      )}

      {/* Persistent footer (§39). */}
      <div className="flex items-center gap-3 pt-5">
        {!skipConfirm ? (
          <button className="text-sm text-faint hover:text-muted" onClick={() => setSkipConfirm(true)}>
            Skip setup
          </button>
        ) : (
          <span className="text-sm text-muted">
            Nothing works without a model —{" "}
            <button className="text-accent" onClick={() => finish()}>
              skip anyway
            </button>
          </span>
        )}
        <button
          className="ml-auto px-6 py-2 rounded-full bg-ink text-panel text-md disabled:opacity-40"
          disabled={!canNext || ps.verify.state === "testing"}
          onClick={advance}
          data-testid="ob-continue"
        >
          {ps.verify.state === "testing" ? "Checking…" : "Next"}
        </button>
      </div>
      <p className="text-xs text-faint mt-3">
        Models can be enabled or hidden anytime in Settings ▸ Models.
      </p>
    </section>
  );
}
