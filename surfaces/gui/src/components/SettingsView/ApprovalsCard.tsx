import { useEffect, useState } from "react";
import { getSettings, getWorkerTypes, setApprovalSettings, type WorkerRoster, type WorkerType } from "../../api";
import { reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";
import { Toggle } from "../Toggle";

// Approvals. The two switches that decide how much happens without you looking —
// deliberately in one card, because that is one question from the user's side, and
// deliberately both off until someone chooses otherwise.
//
// They are not the same kind of switch and the copy has to say so. The read-only one
// is a deterministic list (no network clients, no interpreters, no writes) and its
// worst case is that a command you would have approved anyway runs unasked. The
// reviewer is a model judging intent, and its worst case is a wrong yes — which is
// why the copy names the thing it cannot do rather than promising what it will.
export function ApprovalsCard() {
  const [auto, setAuto] = useState<boolean | null>(null);
  const [readOnly, setReadOnly] = useState<boolean | null>(null);

  useEffect(() => {
    getSettings()
      .then((s) => {
        setAuto(s.auto_approve === true);
        setReadOnly(s.auto_allow_read_only_commands === true);
      })
      .catch(() => {
        setAuto(false);
        setReadOnly(false);
      });
  }, []);

  const save = async (
    next: { auto_approve?: boolean; auto_allow_read_only_commands?: boolean },
    revert: () => void,
  ) => {
    const saved = await reportIfFailed(setApprovalSettings(next), "Couldn't save that setting.");
    if (!saved) revert();
  };

  if (auto === null || readOnly === null) return null;
  return (
    <div className={CARD + " p-4 mb-4"} data-testid="approvals-card">
      <div className={FIELD_LABEL}>Approvals</div>
      {/* No "both are off until you turn them on": it was printed above two boxes whatever
          they were set to, and read as a description of the state. The switches are the state. */}
      <div className={FIELD_HELP}>What can happen without stopping to ask you.</div>
      <div className="flex items-start gap-3 py-2">
        <Toggle
          checked={readOnly}
          label="Run read-only commands without asking"
          testId="readonly-commands-toggle"
          onChange={(next) => {
            setReadOnly(next);
            void save({ auto_allow_read_only_commands: next }, () => setReadOnly(!next));
          }}
        />
        <span>
          <span className="block text-md text-ink">Run read-only commands without asking</span>
          <span className="block text-sm text-muted">
            Things like <code>ls</code>, <code>cat</code> and <code>git log</code>, which look
            at this machine and change nothing. Never anything that writes, and never anything
            that can reach the network &mdash; not even a plain download.
          </span>
        </span>
      </div>
      <div className="flex items-start gap-3 py-2">
        <Toggle
          checked={auto}
          label="Let a reviewer approve routine actions"
          testId="auto-approve-toggle"
          onChange={(next) => {
            setAuto(next);
            void save({ auto_approve: next }, () => setAuto(!next));
          }}
        />
        <span>
          <span className="block text-md text-ink">Let a reviewer approve routine actions</span>
          <span className="block text-sm text-muted">
            A second model checks each action against what you actually asked for, and waves
            through the ones clearly within it. It can only skip a card, never a refusal &mdash;
            anything blocked stays blocked, and anything it is unsure about still comes to you.
          </span>
        </span>
      </div>
    </div>
  );
}

// The team. A lead breaks work into board items and hands each to a worker type;
// this is where you see which types exist and who staffs them.
//
// It exists because the extension point was invisible. Worker types have always
// been user-definable — drop a markdown file into <data dir>/subagents/, complete
// with a README written there on first run — and nothing in the app ever said so.
// A feature nobody can find is a feature nobody has.
//
// Read-only for now, deliberately: editing a type means editing its tool allowlist,
// and a form that lets you widen what a delegate may do without saying what that
// means is worse than the folder.
export function TeamCard() {
  const [data, setData] = useState<{ types: WorkerType[]; rosters: WorkerRoster[] } | null>(null);

  useEffect(() => {
    getWorkerTypes()
      .then(setData)
      .catch(() => setData({ types: [], rosters: [] }));
  }, []);

  if (!data) return null;
  const rosterFor = (name: string) => data.rosters.filter((r) => r.subagents.includes(name));

  return (
    <div className={CARD + " p-4 mb-4"} data-testid="team-card">
      <div className={FIELD_LABEL}>Worker types</div>
      <div className={FIELD_HELP}>
        What a lead assistant can hand work to. Each worker runs one item on its own and reports
        back; a verifier checks it against the item&rsquo;s acceptance criteria before it counts as
        done. Add your own by putting a markdown file in <code>subagents/</code> beside your
        sessions &mdash; the folder has a README.
      </div>

      <div className="mt-3 divide-y divide-line" data-testid="worker-types">
        {data.types.map((t) => {
          const staffed = rosterFor(t.name);
          return (
            <div key={t.name} className="py-2.5">
              <div className="flex items-center gap-2">
                <code className="text-sm text-ink">{t.name}</code>
                {!t.builtin && <span className="text-xs text-muted">yours</span>}
              </div>
              <div className="text-sm text-muted mt-0.5">{t.description}</div>
              <div className="text-xs text-muted mt-1">
                {t.tools === null
                  ? "Uses whatever the assistant that spawned it can use"
                  : `${t.tools.length} tools, fixed`}
                {staffed.length > 0 && ` · staffed by ${staffed.map((r) => r.name).join(", ")}`}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
