import { useEffect, useState } from "react";
import { getSettings, setCompactionSettings, type CompactionSettings } from "../../api";
import { reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";

// -- Context compaction (OPE-27) ------------------------------------------------
// Long sessions are summarized automatically when they approach the model's context
// limit, so work continues instead of hitting a raw provider error. Two spec'd
// overrides (trigger % + token cap) and the summarizer-model pin — nothing more.
export function CompactionCard() {
  const [cfg, setCfg] = useState<CompactionSettings | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [labels, setLabels] = useState<Record<string, string>>({});

  useEffect(() => {
    getSettings()
      .then((s) => {
        setCfg({
          compaction_threshold_pct: s.compaction_threshold_pct ?? 0.8,
          compaction_cap_tokens: s.compaction_cap_tokens ?? 250_000,
          compaction_model: s.compaction_model ?? "",
        });
        setModels(s.models || []);
        setLabels(s.model_labels || {});
      })
      .catch(() =>
        setCfg({
          compaction_threshold_pct: 0.8,
          compaction_cap_tokens: 250_000,
          compaction_model: "",
        }),
      );
  }, []);

  const save = async (patch: Partial<CompactionSettings>) => {
    const before = cfg;
    setCfg((p) => (p ? { ...p, ...patch } : p));
    const saved = await reportIfFailed(setCompactionSettings(patch), "Couldn't save your compaction settings.");
    if (!saved) setCfg(before);
  };

  if (!cfg) return null;
  const modelLabel = (id: string) => labels[id]?.split(" · ")[0] || id;
  return (
    <div className={CARD + " p-4 mb-4"} data-testid="compaction-card">
      <div className={FIELD_LABEL}>Context compaction</div>
      <div className={FIELD_HELP}>
        Long sessions are compacted automatically: older turns are summarized so the
        assistant keeps working instead of running out of context. Your visible transcript
        is never changed — a small marker shows where compaction happened.
      </div>

      <div className="mt-3 flex items-center gap-5 flex-wrap">
        <label className="flex items-center gap-2.5">
          <span className="text-md text-ink">Compact at</span>
          <input
            type="number"
            min={10}
            max={95}
            value={Math.round(cfg.compaction_threshold_pct * 100)}
            data-testid="compaction-threshold"
            className="w-16 px-2 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
            onChange={(e) =>
              save({
                compaction_threshold_pct:
                  Math.max(10, Math.min(Number(e.target.value) || 80, 95)) / 100,
              })
            }
          />
          <span className="text-sm text-muted">% of the context window</span>
        </label>
        <label className="flex items-center gap-2.5">
          <span className="text-md text-ink">or at</span>
          <input
            type="number"
            min={10_000}
            max={2_000_000}
            step={10_000}
            value={cfg.compaction_cap_tokens}
            data-testid="compaction-cap"
            className="w-28 px-2 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
            onChange={(e) =>
              save({
                compaction_cap_tokens: Math.max(
                  10_000,
                  Math.min(Number(e.target.value) || 250_000, 2_000_000),
                ),
              })
            }
          />
          <span className="text-sm text-muted">tokens, whichever is smaller</span>
        </label>
      </div>
      <div className={FIELD_HELP}>
        The cap makes very-large-context models compact early — quality and speed degrade
        well before their nominal limit.
      </div>

      <div className="mt-3 flex items-center gap-2.5">
        <span className="text-md text-ink">Summarizer model</span>
        <select
          value={cfg.compaction_model}
          data-testid="compaction-model"
          className="px-2 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
          onChange={(e) => save({ compaction_model: e.target.value })}
        >
          <option value="">Session&rsquo;s own model (default)</option>
          {models.map((m) => (
            <option key={m} value={m}>
              {modelLabel(m)}
            </option>
          ))}
        </select>
      </div>
      <div className={FIELD_HELP}>
        The summary is written by this model. The default follows whatever model the
        session is using.
      </div>
    </div>
  );
}
