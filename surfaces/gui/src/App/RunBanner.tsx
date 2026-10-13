import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** The banner over an automation run's transcript. */
export function RunBanner({ m }: { m: AppModel }) {
  const { sessionId, runContext, setScheduledOpenId, setSurface } = m;
  return (
    <>{sessionId.startsWith("__run__") && (
              <div
                className="flex items-center gap-2 px-4 py-2 mb-1 rounded-lg text-sm border border-line bg-accentSoft/40"
                data-testid="run-banner"
              >
                <Icon name="clock" size={ICON.md} className="text-accent shrink-0" />
                <span className="truncate text-muted">
                  Scheduled run
                  {runContext?.title ? (
                    <>
                      {" — "}
                      <span className="text-ink font-medium">{runContext.title}</span>
                    </>
                  ) : null}{" "}
                  · started by an automation
                </span>
                <button
                  className="ml-auto shrink-0 text-accent font-medium hover:underline"
                  onClick={() => {
                    if (runContext) setScheduledOpenId(runContext.id);
                    setSurface("scheduled");
                  }}
                >
                  ← Back to runs
                </button>
              </div>
            )}</>
  );
}
