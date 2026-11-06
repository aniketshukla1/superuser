import type { AppModel } from "./model/useAppModel";

/** The toast when a scheduled automation starts, with a way into the run. */
export function RunToast({ m }: { m: AppModel }) {
  const toast = m.runToast;
  if (!toast) return null;
  const close = () => m.setRunToast(null);
  const viewRun = () => {
    m.selectSession(toast.sessionId, toast.workspace, toast.agent);
    close();
  };
  return (
    <div className="bg-panel border border-line fixed pb-2.5 pt-3 px-3.5 right-3 rounded-xl shadow-lg top-3 w-[290px] z-[45]" data-testid="automation-toast">
      <div className="flex font-semibold gap-2 items-center text-sm">
        <span className="bg-faint h-[7px] rounded-full toast-pulse w-[7px]" />
        Automation started
      </div>
      <div className="ml-[15px] mt-0.5 text-muted text-sm truncate">
        {toast.title} · {toast.time} run
      </div>
      <div className="flex items-center justify-between ml-[15px] mt-1.5">
        <button className="font-medium text-accent text-sm" data-testid="toast-view-run" onClick={viewRun}>
          View run ›
        </button>
        <button className="px-0.5 text-faint text-sm" data-testid="toast-dismiss" title="Dismiss" onClick={close}>
          ✕
        </button>
      </div>
      {/* The bar that drains while the toast counts down. */}
      <div className="absolute bg-line bottom-1 h-[2px] left-3 overflow-hidden right-3 rounded">
        <span className="bg-faint block h-full toast-drain" />
      </div>
    </div>
  );
}
