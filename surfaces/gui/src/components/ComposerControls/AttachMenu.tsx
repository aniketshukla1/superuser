import { useState } from "react";
import { Icon, type IconName, ICON } from "../Icon";
import { ChipButton, Menu } from "./MenuRow";

/* ------------------------------------------------------------------ attach */

export function AttachMenu({
  onFiles,
  onFolder,
  onSlashCommands,
  onConnectors,
}: {
  onFiles: () => void;
  onFolder?: () => void;
  onSlashCommands: () => void;
  onConnectors?: () => void;
}) {
  const [open, setOpen] = useState(false);
  const go = (f?: () => void) => () => {
    setOpen(false);
    f?.();
  };
  return (
    <div className="composer-slot is-fixed">
      <ChipButton
        icon="plus"
        open={open}
        onClick={() => setOpen((v) => !v)}
        title="Add files, folders, skills and connectors"
        ariaLabel="Add to this message"
        testid="attach-chip"
      />
      {open && (
        <Menu onClose={() => setOpen(false)} label="Add" width={252} testid="attach-menu">
          <AttachRow icon="paperclip" label="Add files or photos" hint="⌘U" onClick={go(onFiles)} testid="attach-files" />
          {onFolder && <AttachRow icon="folder" label="Add folder" onClick={go(onFolder)} testid="attach-folder" />}
          {/* One name for one thing: this list, the "/" menu and the Settings tab are all
              Skills. It was "Slash commands" here and "Workflows" on the assistant panel. */}
          <AttachRow icon="sparkle" label="Skills" hint="/" onClick={go(onSlashCommands)} testid="attach-slash" />
          {onConnectors && (
            <AttachRow icon="table" label="Connectors" chevron onClick={go(onConnectors)} testid="attach-connectors" />
          )}
        </Menu>
      )}
    </div>
  );
}

export function AttachRow({
  icon,
  label,
  hint,
  chevron,
  soon,
  onClick,
  testid,
}: {
  icon: IconName;
  label: string;
  hint?: string;
  chevron?: boolean;
  soon?: boolean;
  onClick?: () => void;
  testid?: string;
}) {
  return (
    <button
      type="button"
      className={"composer-menu-row is-flat" + (soon ? " is-disabled" : "")}
      onClick={soon ? undefined : onClick}
      disabled={soon}
      role="menuitem"
      data-testid={testid}
    >
      <Icon name={icon} size={ICON.md} className="composer-menu-icon" />
      <span className="composer-menu-name flex-1 text-left">{label}</span>
      {soon && <span className="composer-menu-soon">Coming soon</span>}
      {hint && <kbd className="composer-menu-key">{hint}</kbd>}
      {chevron && <Icon name="chevronRight" size={ICON.md} className="composer-menu-check" />}
    </button>
  );
}
