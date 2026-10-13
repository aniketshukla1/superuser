import { useEffect, useState } from "react";
import { addChiefRoutine, getChief, setChief, type ChiefSetup } from "../api";
import { CARD } from "../ui";

// The Chief of Staff's settings.
//
// A digest of what came in since yesterday — mail, calendars, the apps you've connected, your
// own notes — keeping only what maps to the priorities set here. It never sends a message or
// changes a meeting. What feeds it is shown, so an empty digest can be told from a blind one.

const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export function ChiefOfStaffSection() {
  const [cos, setCos] = useState<ChiefSetup | null>(null);
  const [priorities, setPriorities] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const load = () =>
    getChief().then((c) => {
      setCos(c);
      if (c) setPriorities(c.priorities);
    });
  useEffect(() => {
    load();
  }, []);

  const save = async () => {
    setBusy(true);
    setNotice(null);
    const res = await setChief({ cos_priorities: priorities.trim() });
    setBusy(false);
    if (res.ok === false) {
      setNotice(res.error ?? "Couldn't save that.");
      return;
    }
    await load();
    setNotice("Saved.");
  };

  const addRoutine = async () => {
    setBusy(true);
    setNotice(null);
    const res = await addChiefRoutine();
    setBusy(false);
    if (!res.ok) {
      setNotice(res.error ?? "Couldn't add the routine.");
      return;
    }
    await load();
    setNotice(res.created ? "Added: the Chief of Staff sends you a digest on weekdays at 8:00." : "It's already set up.");
  };

  if (!cos) return null;

  return (
    <section className={`${CARD} p-4 mt-6`} data-testid="chief-of-staff">
      <div className="text-md font-medium">Chief of Staff</div>
      <p className="text-sm text-muted mt-1 mb-3 leading-prose">
        A digest of what came in since yesterday — mail, calendars, the apps you've connected, your
        own notes — keeping only what maps to your priorities, with the next step and whether you
        owe a decision. It never sends a message or changes a meeting.
      </p>

      <label className="text-sm font-medium text-ink" htmlFor="cos-priorities">
        Your priorities
      </label>
      <textarea
        id="cos-priorities"
        className={`${INPUT} mt-1 mb-1 min-h-[72px]`}
        placeholder={"Ship the billing migration by October\nHire two SREs\nKeep on-call load under five pages a week"}
        value={priorities}
        onChange={(e) => setPriorities(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">
        One a line. Everything the digest brings you names the priority it serves.
      </div>

      <div className="text-xs text-muted mb-3" data-testid="cos-sources">
        {cos.sources.length === 0
          ? "Nothing has come in this week yet. Connect your email, calendar or other apps under sidebar ▸ More ▸ Connectors, or keep notes in the Brain."
          : `Reading this week: ${cos.sources.map((s) => `${s.source} (${s.items})`).join(", ")}.`}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <button className={BTN} disabled={busy} onClick={save}>
          {busy ? "Saving…" : "Save"}
        </button>
        {cos.routine ? (
          <span className="text-xs text-muted">A digest runs weekdays at 8:00.</span>
        ) : (
          <button className={BTN_BORDERED} disabled={busy} onClick={addRoutine}>
            Add a morning digest
          </button>
        )}
        {notice ? (
          <span className="text-xs text-muted" role="status">
            {notice}
          </span>
        ) : null}
      </div>
    </section>
  );
}
