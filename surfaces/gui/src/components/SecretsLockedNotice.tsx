// Saved keys that won't open. Without this every provider and connection reads as "not set
// up", and the person re-enters a key that then refuses to save — with no word of why.

import { useState } from "react";
import { setAsideLockedSecrets } from "../api";

const BTN = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export function SecretsLockedNotice({ status, onResolved }: { status: string | undefined; onResolved: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // "Not now" hides it for this run of the app: a local model still works with every key
  // locked, and a notice that can't be put away is one that sits on the composer.
  const [hidden, setHidden] = useState(false);
  if (!status?.startsWith("locked") || hidden) return null;
  const why = status.replace(/^locked:\s*/, "");

  const setAside = async () => {
    setBusy(true);
    setError(null);
    const r = await setAsideLockedSecrets();
    setBusy(false);
    if (r.ok) onResolved();
    else setError(r.error ?? "Couldn't move it.");
  };

  return (
    <div
      role="alert"
      className="fixed top-12 left-1/2 -translate-x-1/2 z-[46] w-[min(560px,calc(100vw-32px))] bg-panel border border-line rounded-xl shadow-lg px-4 py-3"
      data-testid="secrets-locked"
    >
      <div className="text-md font-medium text-ink">Your saved keys can't be opened</div>
      <p className="text-sm text-muted mt-1 leading-prose">
        {why.charAt(0).toUpperCase() + why.slice(1)}. Until this is fixed, every model key and connection reads as
        missing, and nothing new can be saved over them. If the key file (<code>secrets.key</code>) was moved, put it
        back and restart. Otherwise, set the locked file aside — it is kept, not deleted — and enter your keys again.
      </p>
      <div className="flex items-center gap-2 mt-2">
        <button className={BTN} disabled={busy} onClick={() => void setAside()} data-testid="secrets-set-aside">
          {busy ? "Setting aside…" : "Set it aside and start again"}
        </button>
        <button className={BTN} disabled={busy} onClick={() => setHidden(true)} data-testid="secrets-not-now">
          Not now
        </button>
        {error && <span className="text-sm text-danger">{error}</span>}
      </div>
    </div>
  );
}
