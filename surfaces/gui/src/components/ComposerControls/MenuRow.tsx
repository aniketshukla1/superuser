import { useEffect, useRef, type ReactNode } from "react";
import { Icon, type IconName, ICON } from "../Icon";

/* ------------------------------------------------------------------ shared */

/** The chip every control hangs off. Borderless at rest, filled while its menu is open —
 *  so an open panel always has a visible source, which is the thing the old chips got
 *  wrong: you could not tell which one you had opened. */
export function ChipButton({
  label,
  icon,
  open,
  onClick,
  title,
  ariaLabel,
  testid,
  children,
}: {
  label?: string;
  icon?: IconName;
  open: boolean;
  onClick: () => void;
  title: string;
  ariaLabel: string;
  testid?: string;
  children?: ReactNode;
}) {
  return (
    <button
      type="button"
      className={"composer-chip" + (label ? "" : " is-icon") + (open ? " is-open" : "")}
      onClick={onClick}
      title={title}
      aria-label={ariaLabel}
      aria-haspopup="menu"
      aria-expanded={open}
      data-testid={testid}
    >
      {icon && <Icon name={icon} size={ICON.md} />}
      {children}
      {label && <span className="composer-chip-label">{label}</span>}
    </button>
  );
}

/** The panel. One backdrop, one frame, one place that knows menus open upward.
 *
 *  Escape closes it and the trigger keeps focus. That is not decoration: these panels
 *  are opened by pointer and by keyboard, and a keyboard user who opens one with no way
 *  back out is stuck inside the composer. */
export function Menu({
  onClose,
  align = "left",
  width,
  label,
  testid,
  children,
}: {
  onClose: () => void;
  align?: "left" | "right";
  width?: number;
  label: string;
  testid?: string;
  children: ReactNode;
}) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, [onClose]);
  return (
    <>
      <div className="fixed inset-0 z-30" onClick={onClose} data-testid="menu-backdrop" />
      <div
        ref={box}
        className={"composer-menu " + (align === "right" ? "right-0" : "left-0")}
        style={width ? { width } : undefined}
        role="menu"
        aria-label={label}
        data-testid={testid}
      >
        {children}
      </div>
    </>
  );
}

/** The section label above a group of rows ("Mode", "Models", "Microphone"). */
export function MenuHeading({ children }: { children: ReactNode }) {
  return <div className="composer-menu-heading">{children}</div>;
}

export function MenuDivider() {
  return <div className="composer-menu-divider" />;
}

/** One choosable row: a name, an optional line under it, an optional badge, the check
 *  when it is the current one, and the number that picks it from the keyboard. */
export function MenuRow({
  name,
  description,
  badge,
  checked,
  index,
  disabled,
  note,
  onClick,
  trailing,
  testid,
}: {
  name: string;
  description?: string;
  badge?: string;
  checked?: boolean;
  index?: number;
  disabled?: boolean;
  note?: string;
  onClick?: () => void;
  trailing?: ReactNode;
  testid?: string;
}) {
  return (
    <button
      type="button"
      className={"composer-menu-row" + (disabled ? " is-disabled" : "")}
      onClick={disabled ? undefined : onClick}
      disabled={disabled}
      role="menuitemradio"
      aria-checked={!!checked}
      data-testid={testid}
    >
      <span className="composer-menu-main">
        <span className="composer-menu-name">
          {name}
          {badge && <span className="composer-menu-badge">{badge}</span>}
          {note && <span className="composer-menu-note">{note}</span>}
        </span>
        {description && <span className="composer-menu-desc">{description}</span>}
      </span>
      {checked && <Icon name="check" size={ICON.md} className="composer-menu-check" />}
      {trailing}
      {index !== undefined && <kbd className="composer-menu-key">{index}</kbd>}
    </button>
  );
}

/** Number keys pick a row, which is what the badges on them are promising.
 *  Bound while the menu is open and only then — the composer's textarea owns those keys
 *  the rest of the time. */
export function useNumberKeys(count: number, pick: (i: number) => void, on: boolean) {
  useEffect(() => {
    if (!on) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const n = Number(e.key);
      if (!Number.isInteger(n) || n < 1 || n > count) return;
      e.preventDefault();
      e.stopPropagation();
      pick(n - 1);
    };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, [count, pick, on]);
}
