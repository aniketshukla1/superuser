import { useState } from "react";
import { Icon, ICON } from "../Icon";
import { setEffort as apiSetEffort } from "../../api";
import { ChipButton, Menu } from "./MenuRow";

/* ------------------------------------------------------------------ effort */

export const EFFORT_LABEL: Record<string, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  xhigh: "Xhigh",
  max: "Max",
};

export function EffortChip({
  effort,
  levels,
  onChange,
}: {
  effort: string;
  levels: string[];
  onChange: (e: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const i = Math.max(0, levels.indexOf(effort));
  const label = EFFORT_LABEL[effort] || effort;
  const set = (idx: number) => {
    const next = levels[Math.max(0, Math.min(levels.length - 1, idx))];
    if (!next || next === effort) return;
    onChange(next);
    void apiSetEffort(next);
  };
  return (
    <div className="composer-slot">
      <ChipButton
        label={label}
        open={open}
        onClick={() => setOpen((v) => !v)}
        title={`Effort: ${label}`}
        ariaLabel="Effort"
        testid="effort-chip"
      />
      {open && (
        <Menu onClose={() => setOpen(false)} label="Effort" align="right" width={252} testid="effort-menu">
          <div className="composer-effort-head">
            <span className="composer-menu-name">
              Effort <span className="composer-effort-level">{label}</span>
            </span>
            <span
              className="composer-menu-check"
              title="Higher effort gives the model a bigger thinking budget before it answers. Lower is faster."
            >
              <Icon name="helpCircle" size={ICON.md} />
            </span>
          </div>
          <div className="composer-effort-ends">
            <span>Faster</span>
            <span>Smarter</span>
          </div>
          {/* A real range input, not a row of divs: this is a one-dimensional choice and
              the arrow keys should work on it without any of that being reimplemented. */}
          <input
            type="range"
            className="composer-effort-range"
            min={0}
            max={levels.length - 1}
            step={1}
            value={i}
            onChange={(e) => set(Number(e.target.value))}
            aria-label="Effort"
            aria-valuetext={label}
            data-testid="effort-range"
            list="composer-effort-stops"
          />
          <datalist id="composer-effort-stops">
            {levels.map((l) => (
              <option key={l} value={levels.indexOf(l)} label={EFFORT_LABEL[l] || l} />
            ))}
          </datalist>
        </Menu>
      )}
    </div>
  );
}
