// A small on/off switch (mock's `.tgl` / `.knob`) as an accessible button[role=switch]. Driven by
// props so it's testable (query by role "switch", assert aria-checked, fireEvent.click to flip).
// Reused by the persona detail page (default-connection + enable toggles) and the Sources drawer.

export function Toggle({
  checked,
  onChange,
  disabled,
  title,
  label,
  testId,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
  title?: string;
  /** The switch's accessible name, when no visible label is associated with it. */
  label?: string;
  testId?: string;
}) {
  return (
    <button
      type="button"
      className={"tgl" + (checked ? " on" : "")}
      role="switch"
      aria-checked={checked}
      aria-label={label}
      data-testid={testId}
      disabled={disabled}
      title={title}
      onClick={() => {
        if (!disabled) onChange(!checked);
      }}
    >
      <span className="knob" />
    </button>
  );
}
