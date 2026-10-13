import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** The Browser tool button. */
export function BrowserButton({ m }: { m: AppModel }) {
  const { hasBrowser, agent, toolOpen, openTool, toolLive } = m;
  return (
    <>{hasBrowser(agent) && (
              <>
                <button
                  className={"topbar-icon-btn" + (toolOpen("browser") ? " is-on" : "")}
                  aria-pressed={toolOpen("browser")}
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => openTool("browser")}
                  aria-label={toolOpen("browser") ? "Put the browser away" : "Open the browser"}
                  data-tip={
                    toolOpen("browser")
                      ? "Browser — click to put it away"
                      : toolLive("browser")
                        ? "Browser — a page is still open in it"
                        : "Browser"
                  }
                  data-testid="topbar-browser"
                >
                  <Icon name="globe" size={ICON.md} />
                  {toolLive("browser") && !toolOpen("browser") && (
                    <span className="topbar-dot" aria-hidden />
                  )}
                </button>
              </>
            )}</>
  );
}
