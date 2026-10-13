import { useEffect, useState } from "react";
import { forgetFacet, getProfile, pinFacet, saveProfileForm, setProfileLearning, type Facet, type FacetClass, type ProfileForm, type ProfileState } from "../../api";
import { StandingInstructions } from "../StandingInstructions";
import { Toggle } from "../Toggle";
import { CLASSES } from "./CLASSES";
import { FacetRow } from "./FacetRow";
import { ProfileFormEditor } from "./ProfileFormEditor";

export function ProfileSection() {
  const [state, setState] = useState<ProfileState | null>(null);
  const [form, setForm] = useState<ProfileForm | null>(null);
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [showWeighing, setShowWeighing] = useState(false);

  const refresh = () =>
    getProfile()
      .then((p) => {
        setState(p);
        setForm((f) => f ?? p.form);
      })
      .catch(() => setState(null));
  useEffect(() => {
    refresh();
  }, []);

  const apply = (facets: Facet[] | null) => {
    if (facets && state) setState({ ...state, facets });
  };

  const pin = async (f: Facet) => {
    setBusy(true);
    apply(await pinFacet(f, !f.pinned));
    setBusy(false);
  };

  const forget = async (f: Facet) => {
    setBusy(true);
    apply(await forgetFacet(f));
    setBusy(false);
    refresh();
  };

  const allowAgain = async (f: { class: FacetClass; key: string; value: string }) => {
    setBusy(true);
    await forgetFacet(f, false);
    setBusy(false);
    refresh();
  };

  const saveForm = async () => {
    if (!form) return;
    setBusy(true);
    const ok = await saveProfileForm(form);
    setBusy(false);
    setSaved(ok);
    if (ok) refresh();
    setTimeout(() => setSaved(false), 2500);
  };

  const toggleLearning = async () => {
    if (!state) return;
    const next = !state.learning_enabled;
    setState({ ...state, learning_enabled: next });
    await setProfileLearning(next);
  };

  if (!state || !form) {
    return <div className="jrn-empty">Loading…</div>;
  }

  const active = state.facets.filter((f) => f.state === "active");
  const provisional = state.facets.filter((f) => f.state === "provisional");

  return (
    <section className="jrn" data-testid="profile-section">
      <div className="jrn-head">
        <div>
          <h3 className="jrn-title">What it does differently for you</h3>
          <p className="jrn-sub">
            {active.length === 0
              ? "Nothing yet. Fill in the form below, or just talk — anything you say about how you want to be worked with lands here."
              : `${active.length} ${active.length === 1 ? "thing" : "things"} carried into every conversation.` +
                (active.some((f) => !f.from_form)
                  ? " What was picked up from conversation fades unless you say it again — pin the ones that are permanent."
                  : " All of it came from your profile below, so none of it fades.")}
          </p>
        </div>
        {/* A setting, so a switch — it was the one native checkbox on the page, in the platform's blue. */}
        <span className="text-sm text-muted flex items-center gap-2 shrink-0">
          <Toggle
            checked={state.learning_enabled}
            onChange={toggleLearning}
            label="Learn from what I say"
            testId="profile-learning-toggle"
          />
          Learn from what I say
        </span>
      </div>

      {CLASSES.map(({ key, label, blurb }) => {
        const mine = active.filter((f) => f.class === key);
        if (mine.length === 0) return null;
        return (
          <div key={key} className="mem-group" style={{ marginTop: 14 }}>
            <div className="mem-group-head" title={blurb}>
              {label} <span className="mem-group-n">{mine.length}</span>
            </div>
            {mine.map((f) => (
              <FacetRow
                key={`${f.class}:${f.key}:${f.value}`}
                facet={f}
                busy={busy}
                onPin={() => pin(f)}
                onForget={() => forget(f)}
              />
            ))}
          </div>
        );
      })}

      {provisional.length > 0 && (
        <div style={{ marginTop: 18 }}>
          <button
            className="jrn-ghost"
            data-testid="profile-weighing"
            onClick={() => setShowWeighing((v) => !v)}
          >
            {showWeighing ? "Hide" : "Show"} what it's weighing ({provisional.length})
          </button>
          {showWeighing && (
            <>
              <p className="jrn-sub" style={{ marginTop: 8 }}>
                Noticed but not acted on — either there isn't enough evidence yet, or something
                else is winning the same slot. Nothing here reaches a conversation.
              </p>
              {provisional.map((f) => (
                <FacetRow
                  key={`${f.class}:${f.key}:${f.value}`}
                  facet={f}
                  busy={busy}
                  onPin={() => pin(f)}
                  onForget={() => forget(f)}
                />
              ))}
            </>
          )}
        </div>
      )}

      {state.forgotten.length > 0 && (
        <div style={{ marginTop: 18 }}>
          <div className="mem-group-head">Forgotten</div>
          <p className="jrn-sub">Blocked from being learned again.</p>
          {state.forgotten.map((f) => (
            <div
              key={`${f.class}:${f.key}:${f.value}`}
              className="flex items-center gap-3 py-1.5 text-sm text-faint"
            >
              <span className="line-through truncate">{f.value}</span>
              <button className="jrn-ghost ml-auto shrink-0" disabled={busy} onClick={() => allowAgain(f)}>
                Allow again
              </button>
            </div>
          ))}
        </div>
      )}

      <StandingInstructions />

      <ProfileFormEditor state={state} form={form} setForm={setForm} busy={busy} saved={saved} onSave={saveForm} />
    </section>
  );
}
