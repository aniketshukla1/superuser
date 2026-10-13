import { useEffect, useState } from "react";
import { Icon, ICON } from "../Icon";
import { Toggle } from "../Toggle";
import { ChipButton, Menu, MenuDivider, MenuHeading, MenuRow, useNumberKeys } from "./MenuRow";

/* -------------------------------------------------------------------- mode */

export interface ModeOption {
  value: string;
  label: string;
  description: string;
}

/** Permission mode. The dangerous one sits below a divider with its own verb, rather than
 *  third in a list of three — a mode that runs everything without asking should not be
 *  one arrow-key away from the one that asks. */
export function ModeMenu({
  mode,
  options,
  bypass,
  onModeChange,
  unattended,
  onUnattendedChange,
}: {
  mode: string;
  options: ModeOption[];
  bypass?: ModeOption;
  onModeChange: (m: string) => void;
  unattended?: boolean;
  onUnattendedChange?: (on: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  // Turning the guard off asks once. Every other row here is a way of working; this one
  // removes the step where you get to say no, and it was one keystroke away from "Ask me".
  const [confirming, setConfirming] = useState(false);
  const all = bypass ? [...options, bypass] : options;
  const current = all.find((o) => o.value === mode);
  const choose = (i: number) => {
    const o = all[i];
    if (!o) return;
    if (bypass && o.value === bypass.value && mode !== bypass.value) {
      setConfirming(true);
      return;
    }
    onModeChange(o.value);
    setOpen(false);
  };
  useNumberKeys(all.length, choose, open);
  useEffect(() => {
    if (!open) setConfirming(false);
  }, [open]);
  return (
    <div className="composer-slot">
      <ChipButton
        label={current?.label || mode}
        open={open}
        onClick={() => setOpen((v) => !v)}
        title={`Mode: ${current?.label || mode}` + (unattended ? " · approvals go to the Inbox" : "")}
        ariaLabel="Mode"
        testid="mode-chip"
      />
      {open && (
        <Menu onClose={() => setOpen(false)} label="Mode" width={286} testid="mode-menu">
          <MenuHeading>Mode</MenuHeading>
          {options.map((o, i) => (
            <MenuRow
              key={o.value}
              name={o.label}
              description={o.description}
              checked={o.value === mode}
              index={i + 1}
              onClick={() => choose(i)}
              testid={`mode-${o.value}`}
            />
          ))}
          {bypass && (
            <>
              <MenuDivider />
              {/* Named for what it does, and it says "Enable" rather than showing a check,
                  because the others are a choice between ways of working and this is a
                  guard being switched off. In the danger colour for the same reason. */}
              {confirming ? (
                <div className="composer-menu-danger" data-testid="mode-bypass-confirm">
                  <span className="composer-menu-name">
                    <Icon name="alert" size={ICON.md} className="text-danger" />
                    Run everything without asking?
                  </span>
                  <span className="composer-menu-desc">
                    Every command, file edit and message goes ahead unasked, including ones that
                    leave this Mac. It stays off in every other conversation.
                  </span>
                  <span className="flex items-center gap-2 mt-1.5">
                    <button
                      className="btn sm danger"
                      data-testid="mode-bypass-yes"
                      onClick={() => {
                        onModeChange(bypass.value);
                        setConfirming(false);
                        setOpen(false);
                      }}
                    >
                      Turn it off for this chat
                    </button>
                    <button className="btn sm ghost" onClick={() => setConfirming(false)}>
                      Keep asking
                    </button>
                  </span>
                </div>
              ) : (
                <MenuRow
                  name={bypass.label}
                  description={bypass.description}
                  checked={bypass.value === mode}
                  index={options.length + 1}
                  onClick={() => choose(options.length)}
                  trailing={
                    bypass.value === mode ? undefined : (
                      <span className="composer-menu-action is-danger">Enable</span>
                    )
                  }
                  testid={`mode-${bypass.value}`}
                />
              )}
            </>
          )}
          {onUnattendedChange && (
            <>
              <MenuDivider />
              <div className="composer-menu-toggle">
                <span className="min-w-0 flex-1">
                  <span className="composer-menu-name">Send approvals to Inbox</span>
                  <span className="composer-menu-desc">
                    Approvals &amp; questions go to the Inbox; the agent keeps working.
                  </span>
                </span>
                <Toggle checked={!!unattended} onChange={onUnattendedChange} title="Send approvals to the Inbox" />
              </div>
            </>
          )}
        </Menu>
      )}
    </div>
  );
}
