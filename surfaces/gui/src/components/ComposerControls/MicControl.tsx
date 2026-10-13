import { useEffect, useState } from "react";
import { Icon, ICON } from "../Icon";
import { Toggle } from "../Toggle";
import { getVoiceDevices, setVoiceDevice, type InputDevice } from "../../api";
import { ChipButton, Menu, MenuDivider, MenuHeading, MenuRow } from "./MenuRow";

/* --------------------------------------------------------------------- mic */

/** The microphone, plus the arrow that says which one. Two controls in one hit area on
 *  purpose: pressing the mic should record without a decision, and choosing the device is
 *  a decision you make once. */
export function MicControl({
  recording,
  busy,
  ready,
  onToggle,
  reason,
}: {
  recording: boolean;
  busy?: string | null;
  ready: boolean;
  onToggle: () => void;
  reason?: string;
}) {
  const [open, setOpen] = useState(false);
  const [devices, setDevices] = useState<InputDevice[]>([]);
  const [selected, setSelected] = useState("");
  const [present, setPresent] = useState(true);
  const [hold, setHold] = useState(false);

  // Read on open, never cached: an iPhone mic is there when the phone is on the desk and
  // gone when it leaves, so a list from the last time you looked is a list of lies.
  useEffect(() => {
    if (!open) return;
    let live = true;
    void getVoiceDevices().then((v) => {
      if (!live) return;
      setDevices(v.devices);
      setSelected(v.selected);
      setPresent(v.selected_present);
      setHold(v.hold_to_record);
    });
    return () => {
      live = false;
    };
  }, [open]);

  const pick = (name: string) => {
    setSelected(name);
    setPresent(true);
    setOpen(false);
    void setVoiceDevice({ device: name });
  };

  return (
    <div className="composer-slot is-fixed flex items-center">
      {/* Not ready is not the same as disabled. A muted mic still takes the click and
          takes you to the setting that would un-mute it — a control that does nothing
          and does not say why is the worst of the three states. `aria-disabled`, not
          `disabled`, precisely so it stays focusable and clickable while announcing
          that it will not record. */}
      <button
        type="button"
        className={"composer-chip" + (recording ? " is-recording" : "") + (!ready && !recording ? " is-dim" : "")}
        onClick={onToggle}
        disabled={!!busy}
        aria-disabled={!ready && !recording}
        title={busy || (recording ? "Stop recording and transcribe" : ready ? "Start local voice dictation" : reason || "Configure Voice Input in Settings")}
        aria-label={
          recording ? "Stop dictation" : ready ? "Start dictation" : reason || "Configure Voice Input in Settings"
        }
        data-testid="mic-button"
      >
        <Icon name={recording ? "stop" : "mic"} size={ICON.md} />
      </button>
      <div className="composer-slot is-fixed">
        <ChipButton
          icon="chevronDown"
          open={open}
          onClick={() => setOpen((v) => !v)}
          title="Choose a microphone"
          ariaLabel="Microphone options"
          testid="mic-chevron"
        />
        {open && (
          <Menu onClose={() => setOpen(false)} label="Microphone" width={300} testid="mic-menu">
            <MenuHeading>Microphone</MenuHeading>
            <MenuRow
              name="Default"
              description="Follow whatever the system is using"
              checked={selected === ""}
              onClick={() => pick("")}
              testid="mic-default"
            />
            {devices.map((d) => (
              <MenuRow
                key={d.name}
                name={d.name}
                checked={selected === d.name}
                disabled={!d.usable}
                note={d.usable ? undefined : "unavailable"}
                onClick={() => pick(d.name)}
                testid={`mic-device-${d.name}`}
              />
            ))}
            {!present && selected !== "" && (
              // The choice is kept, not silently rewritten: plug it back in and it is
              // used again. Saying so beats a check mark on a row that is not there.
              <div className="composer-menu-hint" data-testid="mic-absent">
                “{selected}” isn’t connected — recording uses the system default until it is.
              </div>
            )}
            <MenuDivider />
            <div className="composer-menu-toggle">
              <span className="composer-menu-name flex-1">Hold to record</span>
              <Toggle
                checked={hold}
                onChange={(v) => {
                  setHold(v);
                  void setVoiceDevice({ hold_to_record: v });
                }}
                title="Hold the mic button to record"
              />
            </div>
          </Menu>
        )}
      </div>
    </div>
  );
}
