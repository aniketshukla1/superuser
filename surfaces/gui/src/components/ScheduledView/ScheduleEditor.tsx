/** Editing an automation's when and on what: time, repeat, and its model. */
export function ScheduleEditor({
  time,
  setTime,
  freq,
  setFreq,
  model,
  setModel,
  models,
  modelLabels,
  pinned,
}: {
  time: string;
  setTime: (t: string) => void;
  freq: string;
  setFreq: (f: string) => void;
  model: string;
  setModel: (m: string) => void;
  models: string[];
  modelLabels: Record<string, string>;
  /** The model this automation is pinned to, if any. */
  pinned?: string | null;
}) {
  return (
    <div className="tmpl-sched sched-edit-sched">
      <label className="tmpl-field">
        <span>At</span>
        <input type="time" className="tmpl-input tmpl-time" value={time} onChange={(e) => setTime(e.target.value)} />
      </label>
      <label className="tmpl-field">
        <span>Repeat</span>
        <select className="tmpl-input tmpl-select" value={freq} onChange={(e) => setFreq(e.target.value)}>
          <option value="daily">Every day</option>
          <option value="weekdays">Weekdays</option>
          <option value="weekends">Weekends</option>
        </select>
      </label>
      {/* Per-automation, so a nightly digest can run on something small while a
          weekly review runs on something capable — without changing a setting
          between them. "Default" follows the app, which is what every existing
          automation does and must keep doing. */}
      <label className="tmpl-field">
        <span>Model</span>
        <select
          className="tmpl-input tmpl-select"
          value={model}
          onChange={(e) => setModel(e.target.value)}
          data-testid="task-model"
        >
          <option value="">Default</option>
          {models.map((m) => (
            <option key={m} value={m}>
              {modelLabels[m] || m}
            </option>
          ))}
          {/* A model this automation is pinned to that is no longer installed
              still has to be shown, or opening the editor would silently move it
              onto the default. */}
          {pinned && !models.includes(pinned) && (
            <option value={pinned}>{modelLabels[pinned] || pinned} (not installed)</option>
          )}
        </select>
      </label>
    </div>
  );
}
