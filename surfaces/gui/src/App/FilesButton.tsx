import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** The Files tool button. */
export function FilesButton({ m }: { m: AppModel }) {
  const { artifactCount, toolOpen, railChosen, setRailHidden, setFilesRequest } = m;
  return (
    <>{artifactCount > 0 && (
              <button
                className={"topbar-icon-btn" + (toolOpen("files") ? " is-on" : "")}
                aria-pressed={toolOpen("files")}
                onMouseDown={(e) => e.stopPropagation()}
                onClick={() => {
                  railChosen.current = true;
                  if (toolOpen("files")) {
                    setRailHidden(true); // already looking at them — put the panel away
                    return;
                  }
                  setRailHidden(false);
                  setFilesRequest((n) => n + 1); // land ON the files, not wherever the panel was
                }}
                // The name says what the NEXT press does, the way the tool buttons do.
                aria-label={
                  toolOpen("files")
                    ? "Hide the files this conversation produced"
                    : `Artifacts — ${artifactCount} file${artifactCount === 1 ? "" : "s"} this conversation produced`
                }
                data-tip={
                  toolOpen("files")
                    ? "Artifacts — click to put the panel away"
                    : `Artifacts — ${artifactCount} file${artifactCount === 1 ? "" : "s"} this conversation produced`
                }
                data-testid="topbar-artifacts"
              >
                <Icon name="file" size={ICON.md} />
                {/* A dot, not a number. "There is something in here" is the whole
                    message; how many is on the tooltip and, properly, in the panel
                    this opens. Hidden while the panel is open on Files, because
                    then you are looking at them and a notification dot for
                    something already on screen is just noise. */}
                {!toolOpen("files") && <span className="topbar-dot" aria-hidden />}
              </button>
            )}</>
  );
}
