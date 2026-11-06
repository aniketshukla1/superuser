import { useEffect, useState } from "react";
import { forgetTalentShortlist, getTalent, pickTalentExport, setTalent, type TalentSetup } from "../api";
import { CARD } from "../ui";

// Talent Scout's settings.
//
// Finds candidates whose public work meets a role's must-haves, leaves out anyone already in the
// person's ATS, and drafts outreach for them to send. Here: the ATS export it checks against
// (read fresh on every check), a message that sounds like them, and the shortlist it keeps —
// which can be forgotten whole.

const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
const BTN_BORDERED = "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40";

export function TalentSection() {
  const [talent, setTalentState] = useState<TalentSetup | null>(null);
  const [voice, setVoice] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const load = () =>
    getTalent().then((t) => {
      setTalentState(t);
      if (t) setVoice(t.voice);
    });
  useEffect(() => {
    load();
  }, []);

  /** One action at a time; what it returns is said beside the buttons. */
  const run = async (work: () => Promise<string | null>) => {
    setBusy(true);
    setNotice(null);
    const said = await work();
    setBusy(false);
    if (said) setNotice(said);
  };

  const save = (patch: { talent_applicants?: string; talent_voice?: string }) =>
    run(async () => {
      const res = await setTalent(patch);
      if (res.ok === false) return res.error ?? "Couldn't save that.";
      await load();
      return "Saved.";
    });

  const choose = () =>
    run(async () => {
      const { path } = await pickTalentExport();
      if (!path) return null;
      const res = await setTalent({ talent_applicants: path });
      if (res.ok === false) return res.error ?? "Couldn't use that file.";
      await load();
      return "Saved.";
    });

  const forget = () =>
    run(async () => {
      const res = await forgetTalentShortlist();
      if (!res.ok) return res.error ?? "Couldn't forget them.";
      await load();
      return `Forgot ${res.forgotten ?? 0}.`;
    });

  if (!talent) return null;

  return (
    <section className={`${CARD} p-4 mt-6`} data-testid="talent">
      <div className="text-md font-medium">Talent Scout</div>
      <p className="text-sm text-muted mt-1 mb-3 leading-prose">
        Finds candidates whose public work meets a role's must-haves, leaves out anyone already in your ATS, and
        drafts outreach for you to send. It contacts no one.
      </p>

      <div className="text-sm font-medium text-ink">Your ATS export</div>
      <div className={`text-xs mt-1 mb-2 ${talent.error ? "text-danger" : "text-muted"}`} data-testid="talent-export">
        {!talent.applicants
          ? "None yet — until you choose one, candidates aren't checked against your applicants."
          : (talent.error ?? `${talent.applicants} — ${talent.people} people.`)}
      </div>
      <div className="flex flex-wrap gap-2 mb-1">
        <button className={BTN_BORDERED} disabled={busy} onClick={choose}>
          {talent.applicants ? "Choose another CSV…" : "Choose a CSV…"}
        </button>
        {talent.applicants && (
          <button className={BTN_BORDERED} disabled={busy} onClick={() => save({ talent_applicants: "" })}>
            Stop using it
          </button>
        )}
      </div>
      <div className="text-xs text-muted mb-3">
        Export candidates as CSV from Greenhouse, Lever, Ashby, Workable or a spreadsheet. It's read fresh on every
        check, so export again to the same place to keep it current.
      </div>

      <label className="text-sm font-medium text-ink" htmlFor="talent-voice">
        A message that sounds like you
      </label>
      <textarea
        id="talent-voice"
        className={`${INPUT} mt-1 mb-1 min-h-[72px]`}
        placeholder="Hi Sam — I read your write-up on the scheduler rewrite…"
        value={voice}
        onChange={(e) => setVoice(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">Paste one you've actually sent. Outreach drafts follow its tone.</div>

      <div className="flex flex-wrap items-center gap-2">
        <button className={BTN} disabled={busy} onClick={() => save({ talent_voice: voice.trim() })}>
          {busy ? "Working…" : "Save"}
        </button>
        {talent.shortlisted > 0 && (
          <button className={BTN_BORDERED} disabled={busy} onClick={forget}>
            Forget the {talent.shortlisted} shortlisted
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
