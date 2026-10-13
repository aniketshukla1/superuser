import { checkBoundaryRule, resetBoundaries, saveBoundaries, testBoundaries, type Boundaries, type BoundaryPreset } from "../../api";
import { PanelHead } from "../IntegrationsView";
import { notifyError } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";
import { BTN_BORDERED, lines, same } from "./INPUT";
import { AllowedRules } from "./AllowedRules";
import { BoundariesActions } from "./BoundariesActions";
import { DenyRulesCard } from "./DenyRulesCard";
import { ModeChooser } from "./ModeChooser";
import { RuleReference } from "./RuleReference";
import { useBoundariesState } from "./useBoundariesState";
import { TestReport } from "./TestReport";

export function BoundariesSection() {
  const {
    state, setState, draft, setDraft, loadFailed, problems, setProblems, checks, setChecks, report,
    setReport, testing, setTesting, saving, setSaving, saved, setSaved, allowText, setAllowText,
    timers, load,
  } = useBoundariesState();

  if (loadFailed) {
    return (
      <section>
        <PanelHead title="Boundaries" sub="What your assistants may never do, in rules you can read." />
        <div className={CARD + " p-4 text-md text-muted"}>Couldn't load your boundaries. Try reopening this page.</div>
      </section>
    );
  }
  if (!state || !draft) {
    return <div className="text-sm text-muted">Loading…</div>;
  }

  const dirty = state.problem ? true : !same(draft, state.boundaries);
  const problemFor = (list: "deny" | "allow", i: number) =>
    problems.find((p) => p.list === list && p.index === i)?.message ?? null;
  const presetById = (id?: string) => state.presets.find((p) => p.id === id);
  const added = new Set(draft.deny.map((r) => r.preset).filter(Boolean));

  const edit = (next: Boundaries) => {
    setDraft(next);
    setSaved(false);
    // A test result describes the rules it was run on, which are no longer these.
    setReport(null);
  };

  const checkLater = (i: number, expr: string) => {
    clearTimeout(timers.current[i]);
    if (!expr.trim()) {
      setChecks((c) => ({ ...c, [i]: "Write the rule, or remove it." }));
      return;
    }
    timers.current[i] = setTimeout(() => {
      void checkBoundaryRule(expr).then((r) => setChecks((c) => ({ ...c, [i]: r.problem })));
    }, 350);
  };

  const setRule = (i: number, patch: { name?: string; when?: string }) => {
    const deny = draft.deny.map((r, j) => {
      if (j !== i) return r;
      const next = { ...r, ...patch };
      // An edited preset is the user's own rule now; it no longer costs what the preset said.
      if (patch.when !== undefined && next.preset && presetById(next.preset)?.when !== patch.when) next.preset = "";
      return next;
    });
    edit({ ...draft, deny });
    setProblems((ps) => ps.filter((p) => !(p.list === "deny" && p.index === i)));
    if (patch.when !== undefined) checkLater(i, patch.when);
  };

  const removeRule = (i: number) => {
    edit({ ...draft, deny: draft.deny.filter((_, j) => j !== i) });
    // Positions after the removed rule shift down; their messages would point at the
    // wrong row, so they are cleared and re-found on the next check or save.
    setChecks({});
    setProblems([]);
  };

  const addPreset = (p: BoundaryPreset) =>
    edit({ ...draft, deny: [...draft.deny, { name: p.name, when: p.when, preset: p.id }] });

  const addBlank = () => edit({ ...draft, deny: [...draft.deny, { name: "", when: "" }] });

  const save = async () => {
    const blank = draft.deny.findIndex((r) => !r.when.trim());
    if (blank >= 0) {
      setChecks((c) => ({ ...c, [blank]: "Write the rule, or remove it." }));
      return;
    }
    setSaving(true);
    const res = await saveBoundaries(draft).catch(() => null);
    setSaving(false);
    if (res?.ok && res.boundaries) {
      setState({ ...state, boundaries: res.boundaries, problem: null, problems: [] });
      setDraft(res.boundaries);
      setProblems([]);
      setSaved(true);
    } else if (res?.problems?.length) {
      setProblems(res.problems);
    } else {
      notifyError("Couldn't save your boundaries.", res?.error);
    }
  };

  const test = async () => {
    setTesting(true);
    const res = await testBoundaries(draft).catch(() => null);
    setTesting(false);
    if (res?.ok && res.report) setReport(res.report);
    else notifyError("Couldn't test those rules.", res?.error);
  };

  const reset = async () => {
    if (!window.confirm("Remove every boundary? Nothing will be refused until you add one again.")) return;
    const res = await resetBoundaries().catch(() => null);
    if (!res?.ok) {
      notifyError("Couldn't reset your boundaries.", res?.error);
      return;
    }
    await load();
    setSaved(true);
  };

  // A rule that can't be read would refuse everything it might cover, so a replay of it
  // would report a boundary far stricter than the one meant. Fixed first, then tested.
  const broken = draft.deny.some((_, i) => checks[i]) || problems.length > 0;

  return (
    <section data-testid="boundaries-section">
      <PanelHead
        title="Boundaries"
        sub="What your assistants may never do, in rules you can read. Checked before every action — before anything is asked, run or sent."
      />

      {state.problem && (
        <div className="rounded-xl2 border border-warnInk bg-warnSoft p-4 mb-4 text-sm text-warnInk" role="alert" data-testid="boundaries-unreadable">
          <div className="font-medium">Your saved boundaries can't be read, so every action is being refused.</div>
          <div className="mt-1">{state.problem}</div>
          <div className="mt-2.5 flex items-center gap-2">
            <button className={BTN_BORDERED} onClick={() => void reset()}>
              Reset to no boundaries
            </button>
            <span>or build a new set below and save it.</span>
          </div>
        </div>
      )}

      <ModeChooser mode={draft.mode} onMode={(mode) => edit({ ...draft, mode })} />

      <DenyRulesCard
        deny={draft.deny}
        presets={state.presets}
        added={added}
        messageFor={(i) => checks[i] ?? problemFor("deny", i)}
        presetById={presetById}
        setRule={setRule}
        removeRule={removeRule}
        addBlank={addBlank}
        addPreset={addPreset}
      />

      <div className={CARD + " p-4 mb-4"} data-testid="boundaries-test">
        <div className="flex items-center gap-3">
          <div className="min-w-0 flex-1">
            <div className={FIELD_LABEL}>Test first</div>
            <div className={FIELD_HELP}>
              Replays these rules — saved or not — over the actions your assistants have already taken, and shows what
              they would have decided differently. Nothing is saved and nothing is re-run.
            </div>
          </div>
          <button className={BTN_BORDERED} disabled={testing || broken} onClick={() => void test()} data-testid="boundaries-test-run">
            {testing ? "Testing…" : "Test these rules"}
          </button>
        </div>
        {report && <TestReport report={report} />}
      </div>

      <AllowedRules
        allowText={allowText}
        allow={draft.allow}
        problemFor={(i) => problemFor("allow", i)}
        onChange={(text) => {
          setAllowText(text);
          edit({ ...draft, allow: lines(text) });
          setProblems((ps) => ps.filter((p) => p.list !== "allow"));
        }}
      />

      <RuleReference fields={state.fields} />

      <BoundariesActions
        dirty={dirty}
        saving={saving}
        saved={saved}
        problemCount={problems.length}
        unreadable={!!state.problem}
        hasRules={state.boundaries.deny.length > 0}
        onSave={() => void save()}
        onDiscard={() => {
          setDraft(state.boundaries);
          setAllowText(state.boundaries.allow.join("\n"));
          setChecks({});
          setProblems(state.problems);
          setReport(null);
        }}
        onReset={() => void reset()}
      />
    </section>
  );
}
