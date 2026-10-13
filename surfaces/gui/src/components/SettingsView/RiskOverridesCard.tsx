import { useEffect, useState } from "react";
import { getSettings, getRiskOverrides, setRiskOverrides, type RiskRule } from "../../api";
import { reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";
import { getCloudStatus, getEgress, getProviders, type CloudStatus, type ProviderInfo } from "../../api";
import { describeDataFlow } from "../../dataFlow";
import { BTN_ACCENT, INPUT, PathText } from "./SettingsView";

// Tool risk. A connected app's tools arrive with no risk information, so the app
// guesses from the name — `search`/`list`/`get` reads, anything else acts. The
// guess is wrong in both directions, and this is where someone corrects it.
//
// The card's real job is the `effective` flag. A rule that asks to loosen a
// built-in is ignored by design, and a rule silently dropped is how someone
// concludes the feature is broken. So an ignored rule is shown, struck through,
// with the reason — it teaches the boundary in one line instead of one support
// conversation.
export function RiskOverridesCard() {
  const [rules, setRules] = useState<RiskRule[] | null>(null);
  const [pattern, setPattern] = useState("");
  const [risk, setRisk] = useState("read");

  const refresh = () => getRiskOverrides().then(setRules).catch(() => setRules([]));
  useEffect(() => {
    refresh();
  }, []);

  const write = async (next: { pattern: string; risk: string }[]) => {
    const res = await setRiskOverrides(next);
    if (res.ok && res.rules) setRules(res.rules);
    else await reportIfFailed(Promise.resolve(res), "Couldn't save that rule.");
  };

  const add = async () => {
    const p = pattern.trim();
    if (!p || !rules) return;
    setPattern("");
    await write([...rules.map((r) => ({ pattern: r.pattern, risk: r.risk })), { pattern: p, risk }]);
  };

  const remove = async (p: string) => {
    if (!rules) return;
    await write(rules.filter((r) => r.pattern !== p).map((r) => ({ pattern: r.pattern, risk: r.risk })));
  };

  if (rules === null) return null;
  return (
    <div className={CARD + " p-4 mb-4"} data-testid="risk-overrides-card">
      <div className={FIELD_LABEL}>Tool risk</div>
      <div className={FIELD_HELP}>
        Connected apps don&rsquo;t say whether a tool reads or acts, so it&rsquo;s guessed from the
        name. Correct it here. Patterns match tool names and may use <code>*</code>; the most
        specific rule wins.
      </div>

      {rules.length > 0 && (
        <div className="mt-3 divide-y divide-line" data-testid="risk-rules">
          {rules.map((r) => (
            <div key={r.pattern} className="py-2 flex items-center gap-3">
              <code className={"text-sm flex-1 break-all " + (r.effective ? "text-ink" : "text-muted line-through")}>
                {r.pattern}
              </code>
              <span className="text-sm text-muted">{r.risk}</span>
              {!r.effective && (
                <span className="text-xs text-muted" title="A rule may tighten a built-in tool, never loosen one.">
                  ignored &mdash; can&rsquo;t loosen a built-in
                </span>
              )}
              <button className="text-sm text-red-600 px-2 py-1" onClick={() => void remove(r.pattern)}>
                Remove
              </button>
            </div>
          ))}
        </div>
      )}

      <div className="flex items-center gap-2 mt-3">
        <input
          className={INPUT}
          type="text"
          placeholder="notion_*"
          value={pattern}
          spellCheck={false}
          autoComplete="off"
          data-testid="risk-pattern"
          onChange={(e) => setPattern(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && void add()}
        />
        <select className={INPUT} value={risk} data-testid="risk-class" onChange={(e) => setRisk(e.target.value)}>
          <option value="read">read</option>
          <option value="write_local">write_local</option>
          <option value="exec">exec</option>
          <option value="external">external</option>
        </select>
        <button className={BTN_ACCENT} disabled={!pattern.trim()} onClick={() => void add()}>
          Add
        </button>
      </div>
    </div>
  );
}

// Privacy & data. This is the one card where we can say something Energy can't:
// their Privacy Mode is a toggle ("Energy only stores your data locally"), ours is
// how the app is built. The card states plainly where the data lives and — the part
// that matters — is honest that enabling a cloud model DOES send prompts off the
// machine. A privacy panel that hid that would be worse than none.
export function PrivacyCard() {
  const [paths, setPaths] = useState<{ scratch?: string; secrets?: string; version?: string }>({});
  // The statement is computed from what is configured (dataFlow.ts), not written here. This
  // card used to assert "no account, no sync, and no telemetry" and "every prompt is answered
  // by a model running on this machine" while a cloud provider was connected and a cloud
  // sign-in sat one menu away.
  const [providers, setProviders] = useState<ProviderInfo[] | null>(null);
  const [cloud, setCloud] = useState<CloudStatus | null>(null);
  const [localOnly, setLocalOnly] = useState<boolean | null>(null);

  useEffect(() => {
    getSettings()
      .then((s: any) =>
        setPaths({
          scratch: s.scratch_base,
          secrets: s.secrets_path,
          version: s.version,
        }),
      )
      .catch(() => {});
    getProviders().then((p) => setProviders(Array.isArray(p) ? p : [])).catch(() => setProviders(null));
    getCloudStatus().then(setCloud).catch(() => setCloud(null));
    getEgress().then((e) => setLocalOnly(e.local_only)).catch(() => setLocalOnly(null));
  }, []);

  const facts = describeDataFlow({ providers, cloud, localOnly });

  return (
    <div className={CARD + " p-4 mb-4"} data-testid="privacy-card">
      <div className={FIELD_LABEL + " mb-1"}>Where your data goes</div>
      <ul className="data-facts">
        {facts.map((f) => (
          <li key={f.text} className={"data-fact is-" + f.tone}>
            {f.text}
          </li>
        ))}
      </ul>

      <div className="mt-3 space-y-1.5">
        <div className="flex items-baseline gap-2 text-sm">
          <span className="text-muted w-[86px] shrink-0">Sessions</span>
          {paths.scratch ? <PathText path={paths.scratch} /> : <code className="text-ink">—</code>}
        </div>
        <div className="flex items-baseline gap-2 text-sm">
          <span className="text-muted w-[86px] shrink-0">Credentials</span>
          {paths.secrets ? <PathText path={paths.secrets} /> : <code className="text-ink">—</code>}
        </div>
      </div>

      <a className="data-facts-link" href="#/activity">
        See everything that has left this Mac ›
      </a>

      {paths.version && (
        <div className="mt-4 pt-3 border-t border-line text-xs text-faint">
          Superuser {paths.version} · Beta
        </div>
      )}
    </div>
  );
}
