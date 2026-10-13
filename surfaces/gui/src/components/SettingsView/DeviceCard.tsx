import type { DictationStatus } from "../../tauri";
import { Icon, ICON } from "../Icon";
import { CARD } from "../../ui";

/** This device: whether local transcription runs here, and the published floor. */
export function DeviceCard({ status }: { status: DictationStatus | null }) {
  return (
    <div className={CARD}>
      <div className="p-4 flex items-start gap-3">
        <Icon name="code" size={ICON.lg} className="text-accent mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="text-md font-medium">This device</div>
          <div className="text-sm text-muted mt-1">{status?.device_summary || "Checking compatibility…"}</div>
          {status?.compatibility_reason && <div className="text-sm text-red-600 mt-1.5">{status.compatibility_reason}</div>}
        </div>
        {status && (
          <span className={"text-xs px-2 py-1 rounded-full " + (status.supported ? "bg-green-50 text-green-700" : "bg-red-50 text-red-600")}>
            {status.supported ? "● Compatible" : "Unsupported"}
          </span>
        )}
      </div>
      <div className="border-t border-line bg-paper/50 px-4 py-3 grid grid-cols-2 gap-3 text-sm text-muted">
        <div><span className="block text-ink font-medium">Mac</span>macOS 12+ · Apple Silicon M1+</div>
        <div><span className="block text-ink font-medium">Windows</span>Windows 10 22H2/11 · x64</div>
        <div><span className="block text-ink font-medium">Memory</span>8 GB recommended</div>
        <div><span className="block text-ink font-medium">Processor</span>4 CPU cores recommended</div>
      </div>
    </div>
  );
}
