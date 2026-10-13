import { useEffect, useState } from "react";
import { calibrateVoice, getVoiceCalibration, type VoiceCalibration } from "../../tauri";
import { Icon, ICON } from "../Icon";
import { CARD } from "../../ui";
import { BTN_BORDERED } from "./SettingsView";
import { voiceError } from "./formatBytes";

/** Room calibration: how quiet this room gets, so hands-free knows when you stopped. */
export function RoomCalibrationCard({ supported, setError }: { supported: boolean; setError: (e: string | null) => void }) {
  // The room measurement. Made here, once, rather than at the start of every
  // conversation: thresholds only mean something relative to the room they are
  // used in, and measuring silently at session start both cost dead air and
  // assumed the user happened to be quiet at that moment.
  const [calibration, setCalibration] = useState<VoiceCalibration | null>(null);
  const [calibrating, setCalibrating] = useState(false);

  useEffect(() => {
    void getVoiceCalibration().then((c) => setCalibration(c ?? null));
  }, []);

  const calibrate = async () => {
    setError(null);
    setCalibrating(true);
    try {
      setCalibration(await calibrateVoice());
    } catch (calibrateError) {
      setError(voiceError(calibrateError));
    } finally {
      setCalibrating(false);
    }
  };

  return (
    <div className={CARD}>
      <div className="p-4 flex items-center gap-3">
        <Icon name="sparkle" size={ICON.lg} className={calibration ? "text-green-600" : "text-muted"} />
        <div className="min-w-0 flex-1">
          <div className="text-md font-medium">Room calibration</div>
          <div className="text-sm text-muted mt-0.5">
            {calibrating
              ? "Measuring — stay quiet for a moment."
              : calibration
                ? "Hands-free listening is tuned to this room."
                : "Hands-free needs to know how quiet this room gets, so it can tell when you have stopped talking."}
          </div>
        </div>
        <button
          className={BTN_BORDERED}
          disabled={!supported || calibrating}
          onClick={() => void calibrate()}
          data-testid="calibrate-voice"
        >
          {calibrating ? "Measuring…" : calibration ? "Measure again" : "Measure this room"}
        </button>
      </div>
      {calibration && (
        // The numbers are shown because they are the thing that goes wrong:
        // when hands-free misbehaves, these say which way.
        <div className="border-t border-line bg-paper/50 px-4 py-3 text-sm text-muted">
          Room noise <b className="text-ink">{calibration.noise_floor.toFixed(3)}</b> · starts listening above{" "}
          <b className="text-ink">{calibration.start_level.toFixed(3)}</b> · counts silence below{" "}
          <b className="text-ink">{calibration.end_level.toFixed(3)}</b>
          <div className="mt-1">Measure again if you move rooms or change microphone.</div>
        </div>
      )}
    </div>
  );
}
