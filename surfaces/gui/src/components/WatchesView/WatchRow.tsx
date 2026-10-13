import { useState } from "react";
import { deleteWatch, setWatchEnabled, type Watch } from "../../api";
import { Icon, ICON } from "../Icon";
import { Toggle } from "../Toggle";
import { CARD } from "../../ui";
import { relative } from "./AutomationsSurface";

export function WatchRow({ w, onChanged }: { w: Watch; onChanged: () => void }) {
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);

  const toggle = async () => {
    setBusy(true);
    await setWatchEnabled(w.id, !w.enabled);
    setBusy(false);
    onChanged();
  };
  const remove = async () => {
    if (!confirm) {
      setConfirm(true);
      return;
    }
    setBusy(true);
    await deleteWatch(w.id);
    setBusy(false);
    onChanged();
  };

  return (
    <div className={CARD + " p-3 flex items-center gap-3"} data-testid="watch-row">
      <div className="flex-1 min-w-0">
        <div className={"text-md truncate " + (w.enabled ? "text-ink" : "text-faint")}>
          {w.question}
        </div>
        <div className="text-sm text-faint mt-0.5">
          {w.sources.length ? w.sources.join(", ") : "everything"}
          {" · "}
          {w.last_tick ? `last looked ${relative(w.last_tick)}` : "not looked yet"}
        </div>
      </div>
      {/* A switch, like an automation's Active/Paused. It was a button reading "On", which says
          the state and leaves you to guess that pressing it is how you change it. */}
      <Toggle
        checked={w.enabled}
        onChange={toggle}
        disabled={busy}
        label={w.enabled ? "Watching — switch off" : "Paused — switch on"}
        title={w.enabled ? "Watching" : "Paused"}
        testId="watch-toggle"
      />
      <button
        className={confirm ? "btn sm danger shrink-0" : "nav-icon-btn shrink-0"}
        disabled={busy}
        onClick={remove}
        onBlur={() => setConfirm(false)}
        title="Delete this watch"
        aria-label={confirm ? "Confirm delete" : "Delete this watch"}
        data-testid="watch-delete"
      >
        {confirm ? "Delete?" : <Icon name="trash" size={ICON.md} />}
      </button>
    </div>
  );
}
