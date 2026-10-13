import { useEffect, useState } from "react";
import { getSettings, setGenerativeUi, setStallLimits } from "../../api";
import { notifyError, reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";

// -- When a model goes quiet ------------------------------------------------------
// A provider that accepted a turn and then sent nothing used to hold it open forever,
// behind a spinner that looked exactly like thinking. The limit is on SILENCE, not on
// how long a turn takes, so a long answer that keeps arriving is never cut off. Saved
// on blur rather than per keystroke: the server floors a limit at ten seconds, and
// typing "60" one digit at a time would otherwise pass through a saved "10".
export function StallCard() {
  const [cloud, setCloud] = useState<string | null>(null);
  const [local, setLocal] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then((s) => {
        setCloud(String(s.stall_seconds ?? 60));
        setLocal(String(s.stall_seconds_local ?? 300));
      })
      .catch(() => {
        setCloud("60");
        setLocal("300");
      });
  }, []);

  const commit = async (key: "stall_seconds" | "stall_seconds_local", raw: string) => {
    const n = Math.floor(Number(raw));
    if (!Number.isFinite(n) || n < 0) return;
    const res = await setStallLimits({ [key]: n }).catch(() => null);
    if (!res?.ok) {
      notifyError("Couldn't save that limit.", res?.error);
      return;
    }
    // What the server kept, which is not always what was typed.
    setCloud(String(res.stall_seconds ?? cloud));
    setLocal(String(res.stall_seconds_local ?? local));
  };

  if (cloud === null || local === null) return null;
  const field = (
    label: string,
    value: string,
    set: (v: string) => void,
    key: "stall_seconds" | "stall_seconds_local",
  ) => (
    <label className="flex items-center gap-2.5">
      <span className="text-md text-ink">{label}</span>
      <input
        type="number"
        min={0}
        max={3600}
        value={value}
        data-testid={`stall-${key}`}
        className="w-20 px-2 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
        onChange={(e) => set(e.target.value)}
        onBlur={(e) => void commit(key, e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && void commit(key, (e.target as HTMLInputElement).value)}
      />
      <span className="text-sm text-muted">{value === "0" ? "off" : "seconds"}</span>
    </label>
  );
  return (
    <div className={CARD + " p-4 mb-4"} data-testid="stall-card">
      <div className={FIELD_LABEL}>When a model goes quiet</div>
      <div className={FIELD_HELP}>
        If a model sends nothing at all for this long, the turn ends and says so instead of waiting forever. A long
        answer that keeps arriving is never cut off. 0 switches this off.
      </div>
      <div className="mt-3 flex items-center gap-5 flex-wrap">
        {field("Cloud models", cloud, setCloud, "stall_seconds")}
        {field("Models on this Mac", local, setLocal, "stall_seconds_local")}
      </div>
      <div className={FIELD_HELP}>
        Models on this Mac get longer: loading one and reading a long prompt are slow and silent on a laptop.
      </div>
    </div>
  );
}

// Views an assistant draws for itself (Phase 5, G1). Off by default, and the copy says
// both halves: what it can do, and what it cannot — because "the model wrote this page"
// is the part a person needs to know while they are looking at one.
export function GenerativeUiCard() {
  const [on, setOn] = useState<boolean | null>(null);

  useEffect(() => {
    getSettings()
      .then((s) => setOn(s.generative_ui === true))
      .catch(() => setOn(false));
  }, []);

  const save = async (next: boolean) => {
    setOn(next);
    const saved = await reportIfFailed(setGenerativeUi(next), "Couldn't save that setting.");
    if (!saved) setOn(!next);
  };

  if (on === null) return null;
  return (
    <div className={CARD + " p-4 mb-4"} data-testid="generative-ui-card">
      <div className={FIELD_LABEL}>Views an assistant draws</div>
      <label className="flex items-start gap-3 py-2">
        <input
          type="checkbox"
          className="mt-0.5"
          data-testid="generative-ui-toggle"
          checked={on}
          onChange={(e) => void save(e.target.checked)}
        />
        <span>
          <span className="block text-md text-ink">Let assistants lay out a small view of their own</span>
          <span className={"block " + FIELD_HELP}>
            For answers that don&rsquo;t fit a card, a table or a chart. What they draw runs sealed: its
            own frame, no network, no reach into this app or anything you have signed into, and nothing
            in it can send anything anywhere. It is still a page a model wrote, which is why this is off
            until you turn it on.
          </span>
        </span>
      </label>
    </div>
  );
}
