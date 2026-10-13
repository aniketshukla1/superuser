import type { AppModel } from "./model/useAppModel";

/** The toast for a finished background run. */
export function RunToast({ m }: { m: AppModel }) {
  const { runToast, selectSession, setRunToast } = m;
  return (
    <>{runToast && (
        <div
          className="fixed top-3 right-3 z-[45] w-[290px] bg-panel border border-line rounded-xl shadow-lg px-3.5 pt-3 pb-2.5"
          data-testid="automation-toast"
        >
          <div className="flex items-center gap-2 text-sm font-semibold">
            <span className="w-[7px] h-[7px] rounded-full bg-faint toast-pulse" />
            Automation started
          </div>
          <div className="text-sm text-muted mt-0.5 ml-[15px] truncate">
            {runToast.title} · {runToast.time} run
          </div>
          <div className="flex items-center justify-between ml-[15px] mt-1.5">
            <button
              className="text-sm text-accent font-medium"
              data-testid="toast-view-run"
              onClick={() => {
                selectSession(runToast.sessionId, runToast.workspace, runToast.agent);
                setRunToast(null);
              }}
            >
              View run ›
            </button>
            <button
              className="text-sm text-faint px-0.5"
              data-testid="toast-dismiss"
              title="Dismiss"
              onClick={() => setRunToast(null)}
            >
              ✕
            </button>
          </div>
          <div className="absolute left-3 right-3 bottom-1 h-[2px] rounded bg-line overflow-hidden">
            <span className="block h-full bg-faint toast-drain" />
          </div>
        </div>
      )}</>
  );
}
