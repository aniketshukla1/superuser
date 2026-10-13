// Settings ▸ General: the accent and the finish (src/accent.ts). Swatches for the curated set,
// "Theme" for each theme's own accent, and a custom colour from the system's colour panel.
// Whatever is picked is fitted to the theme on screen, so the help text can promise that any
// colour works.
import { useEffect, useState } from "react";
import {
  ACCENTS,
  ACCENT_EVENT,
  type AccentPref,
  type Finish,
  getAccentPref,
  getFinish,
  setAccentPref,
  setFinish,
} from "../accent";
import { FIELD_HELP, FIELD_LABEL } from "../ui";

function useAccent(): [AccentPref, Finish] {
  const [state, setState] = useState<[AccentPref, Finish]>(() => [getAccentPref(), getFinish()]);
  useEffect(() => {
    const sync = () => setState([getAccentPref(), getFinish()]);
    window.addEventListener(ACCENT_EVENT, sync);
    return () => window.removeEventListener(ACCENT_EVENT, sync);
  }, []);
  return state;
}

export function AccentPicker() {
  const [pref, finish] = useAccent();
  const custom = pref.startsWith("#") ? pref : null;

  const swatch = (id: AccentPref, name: string, color: string | null, extra = "") => (
    <button
      key={id}
      type="button"
      role="radio"
      aria-checked={pref === id}
      aria-label={name}
      data-tip={name}
      data-testid={`accent-${id}`}
      className={"accent-swatch" + extra + (pref === id ? " is-on" : "")}
      style={color ? { ["--sw" as string]: color } : undefined}
      onClick={() => setAccentPref(id)}
    />
  );

  return (
    <div data-testid="accent-picker">
      <div className={FIELD_LABEL}>Accent</div>
      <div className={FIELD_HELP + " mb-3"}>
        Buttons, links, the open tab and the brand mark. Any colour works: it is adjusted so text on
        it stays readable in every theme.
      </div>
      <div className="accent-row">
        <span className="accent-row" role="radiogroup" aria-label="Accent colour">
          {swatch("theme", "Theme's own", null, " is-theme")}
          {ACCENTS.map((a) => swatch(a.id, a.name, a.hex))}
        </span>
        <label
          className={"accent-swatch is-custom" + (custom ? " is-on is-set" : "")}
          data-tip={custom ? `Custom ${custom}` : "Custom colour"}
          style={custom ? { ["--sw" as string]: custom } : undefined}
        >
          <input
            type="color"
            aria-label="Custom accent colour"
            data-testid="accent-custom"
            value={(custom ?? "#7CC4FF").toLowerCase()}
            onChange={(e) => setAccentPref(e.target.value.toUpperCase() as AccentPref)}
          />
        </label>
      </div>

      <div className={FIELD_LABEL + " mt-5"}>Finish</div>
      <div className="seg mt-2.5" role="radiogroup" aria-label="Finish">
        {(["flat", "metal"] as const).map((f) => (
          <button
            key={f}
            type="button"
            role="radio"
            aria-checked={finish === f}
            data-testid={`finish-${f}`}
            className={finish === f ? "active" : ""}
            onClick={() => setFinish(f)}
          >
            {f === "flat" ? "Flat" : "Metal"}
          </button>
        ))}
      </div>
      <div className={FIELD_HELP}>Metal brushes the buttons, the titles and the brand mark; flat keeps them plain.</div>
    </div>
  );
}
