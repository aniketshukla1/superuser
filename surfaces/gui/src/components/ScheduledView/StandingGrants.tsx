import type { Automation } from "../../api";

type Rule = NonNullable<Automation["always_allowed"]>[number];

/** Standing approvals this automation may use, each revocable. */
export function StandingGrants({ rules, onRevoke }: { rules: Rule[]; onRevoke: (entry: string) => void }) {
  if (rules.length === 0) return null;
  return (
    <>
      <div className="sa-sub">Allowed without asking</div>
      <div className="text-sm text-faint mb-2">
        Standing approvals this automation may use — everything else still asks first.
      </div>
      <div className="sched-grants" data-testid="task-grants">
        {rules.map((rule) => (
          <div className="sched-grant" key={rule.entry}>
            <span className="sched-grant-rule">
              <code>{rule.tool}</code>
              {rule.target && <span className="sched-grant-target"> → {rule.target}</span>}
            </span>
            <button
              className="link"
              title="This automation will ask for approval again"
              onClick={() => onRevoke(rule.entry)}
            >
              Revoke
            </button>
          </div>
        ))}
      </div>
    </>
  );
}
