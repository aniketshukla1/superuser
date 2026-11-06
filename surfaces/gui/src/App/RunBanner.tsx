import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** Over the transcript of an automation's run: which automation, and the way back to its runs. */
export function RunBanner({ m }: { m: AppModel }) {
  if (!m.sessionId.startsWith("__run__")) return null;
  const task = m.runContext;
  const backToRuns = () => {
    if (task) m.setScheduledOpenId(task.id);
    m.setSurface("scheduled");
  };
  return (
    <div className="bg-accentSoft/40 border border-line flex gap-2 items-center mb-1 px-4 py-2 rounded-lg text-sm" data-testid="run-banner">
      <Icon name="clock" size={ICON.md} className="shrink-0 text-accent" />
      <span className="text-muted truncate">
        Scheduled run
        {task?.title ? (
          <>
            {" — "}
            <span className="font-medium text-ink">{task.title}</span>
          </>
        ) : null}{" "}
        · started by an automation
      </span>
      <button className="font-medium hover:underline ml-auto shrink-0 text-accent" onClick={backToRuns}>
        ← Back to runs
      </button>
    </div>
  );
}
