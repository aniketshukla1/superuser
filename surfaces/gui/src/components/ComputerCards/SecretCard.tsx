import { useEffect, useRef, useState } from "react";
import { declineComputerSecret, supplyComputerSecret } from "../../api";
import { Icon, ICON } from "../Icon";
import { BTN_ACCENT, BTN_BORDERED, useLapsed } from "./ComputerHelpCard";

export function SecretCard({
  persona,
  name,
  label,
  onDone,
  onEnded,
  compact,
}: {
  persona: string;
  name: string;
  label: string;
  onDone: (result: "sent" | "declined") => void;
  /** The request is no longer open in the browser. */
  onEnded?: () => void;
  compact?: boolean;
}) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [sent, setSent] = useState<number | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  useLapsed(persona, (c) => !!c.secret_wanted, onEnded);
  useEffect(() => {
    const t = window.setTimeout(() => inputRef.current?.focus(), 60);
    return () => clearTimeout(t);
  }, []);

  const submit = async () => {
    if (busy || !value) return;
    setBusy(true);
    setError("");
    const res = await supplyComputerSecret(persona, value).catch(() => null);
    setBusy(false);
    if (!res?.ok) {
      // Kept, so a transient failure can be retried without typing it again.
      setError(res?.error || "It didn't reach the page. Try again.");
      return;
    }
    // Gone the moment it has landed.
    setValue("");
    setSent(res.chars ?? 0);
    onDone("sent");
  };

  const decline = async () => {
    setBusy(true);
    setValue("");
    await declineComputerSecret(persona).catch(() => null);
    setBusy(false);
    onDone("declined");
  };

  if (sent !== null)
    return (
      <div className="rounded-xl2 border border-line bg-panel p-3 text-sm text-ink" data-testid="secret-card-sent">
        Entered — {sent} character{sent === 1 ? "" : "s"} went into the page.
      </div>
    );

  return (
    <form
      className={"rounded-xl2 border border-line bg-panel p-3" + (compact ? "" : " max-w-[560px]")}
      data-testid="secret-card"
      aria-label={`${name} is asking for ${label}`}
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <div className="flex items-start gap-2.5">
        <Icon name="shield" size={ICON.lg} className="text-accent shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="text-md font-medium text-ink">
            {name} is asking for {label}
          </div>
          <p className="text-sm text-muted mt-1 leading-prose">
            It goes straight into the page. {name} never sees it, and it isn&rsquo;t saved anywhere.
          </p>
        </div>
      </div>
      <div className="flex items-center gap-2 mt-3">
        <input
          ref={inputRef}
          type="password"
          autoComplete="off"
          spellCheck={false}
          aria-label={label}
          className="flex-1 min-w-0 px-3 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
          value={value}
          disabled={busy}
          onChange={(e) => setValue(e.target.value)}
          data-testid="secret-card-input"
        />
        <button className={BTN_ACCENT} type="submit" disabled={busy || !value} data-testid="secret-card-send">
          Enter it
        </button>
        <button className={BTN_BORDERED} type="button" disabled={busy} onClick={() => void decline()} data-testid="secret-card-decline">
          Don&rsquo;t
        </button>
      </div>
      {error && <p className="text-sm text-danger mt-2">{error}</p>}
    </form>
  );
}
