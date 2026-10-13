// Settings ▸ General ▸ Theme: Auto plus sixteen themes. Every card is painted in
// its own theme — the swatch carries `data-palette`, and the palette blocks in styles.css
// apply to `.theme-preview` as well as to <html> — so what you see is what you get.
//
// A radio group: one tab stop (the chosen card), arrow keys move the choice, and choosing
// applies at once — there is nothing to confirm, and Auto is always one card away.
import { useRef } from "react";
import { AUTO_THEME, THEMES, type ThemeId, type ThemePref, useThemePref } from "../theme";
import { Icon, ICON } from "./Icon";
import { SEC_H } from "../ui";

function Swatch({ id }: { id: ThemeId }) {
  return (
    <span className="theme-preview" data-palette={id} aria-hidden="true">
      <span className="tp-side">
        <i className="tp-line" />
        <i className="tp-line is-short" />
      </span>
      <span className="tp-main">
        <span className="tp-card">
          <i className="tp-line" />
          <i className="tp-line is-muted" />
        </span>
        <i className="tp-cta" />
      </span>
    </span>
  );
}

const OPTIONS: { id: ThemePref; name: string; note: string }[] = [
  { id: "auto", name: "Auto", note: "follows your Mac" },
  ...THEMES,
];

export function ThemePicker() {
  const [pref, setPref] = useThemePref();
  const cards = useRef<(HTMLButtonElement | null)[]>([]);

  const onKey = (e: React.KeyboardEvent, i: number) => {
    const step = ({ ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 } as Record<string, number>)[e.key];
    if (!step) return;
    e.preventDefault();
    const next = (i + step + OPTIONS.length) % OPTIONS.length;
    setPref(OPTIONS[next].id);
    cards.current[next]?.focus();
  };

  const card = (o: (typeof OPTIONS)[number]) => {
    const i = OPTIONS.indexOf(o);
    const on = o.id === pref;
    return (
      <button
        key={o.id}
        ref={(el) => {
          cards.current[i] = el;
        }}
        type="button"
        role="radio"
        aria-checked={on}
        tabIndex={on ? 0 : -1}
        data-testid={`theme-${o.id}`}
        className={"theme-card" + (on ? " is-on" : "")}
        onClick={() => setPref(o.id)}
        onKeyDown={(e) => onKey(e, i)}
      >
        {o.id === "auto" ? (
          <span className="theme-auto">
            <Swatch id={AUTO_THEME.light} />
            <Swatch id={AUTO_THEME.dark} />
          </span>
        ) : (
          <Swatch id={o.id} />
        )}
        <span className="tc-foot">
          <span className="tc-name">{o.name}</span>
          <span className="tc-note">{o.note}</span>
        </span>
        {on && (
          <span className="tc-check" aria-hidden="true">
            <Icon name="check" size={ICON.sm} />
          </span>
        )}
      </button>
    );
  };

  return (
    <div className="theme-picker" role="radiogroup" aria-label="Theme">
      <div className="theme-grid">{card(OPTIONS[0])}</div>
      {(["dark", "light"] as const).map((mode) => (
        <div key={mode}>
          <div className={SEC_H + " mb-2"}>{mode === "dark" ? "Dark" : "Light"}</div>
          <div className="theme-grid">{OPTIONS.filter((o) => o.id !== "auto" && THEMES.find((t) => t.id === o.id)!.mode === mode).map(card)}</div>
        </div>
      ))}
    </div>
  );
}
