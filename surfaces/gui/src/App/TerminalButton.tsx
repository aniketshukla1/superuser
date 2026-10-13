import { Icon, ICON } from "../components/Icon";
import type { AppModel } from "./model/useAppModel";

/** The Terminal tool button. */
export function TerminalButton({ m }: { m: AppModel }) {
  const { hasShell, agent, toolOpen, openTool, sharesShell, toolLive } = m;
  return (
    <>{hasShell(agent) && (
              <>
                <button
                  className={"topbar-icon-btn" + (toolOpen("terminal") ? " is-on" : "")}
                  aria-pressed={toolOpen("terminal")}
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => openTool("terminal")}
                  // The name says what the NEXT press does. The button is a toggle, so a
                  // fixed "Open the terminal" is wrong half the time — and to a screen
                  // reader it is the only half there is.
                  aria-label={toolOpen("terminal") ? "Put the terminal away" : "Open the terminal"}
                  // "The shell this session shares" is only true when the agent can
                  // actually run a command in it. On an assistant it is your shell
                  // and yours alone, and saying otherwise would have the user believe
                  // the agent is about to type — see `agentSharesShell`.
                  data-tip={
                    toolOpen("terminal")
                      ? "Terminal — click to put it away (the shell keeps running)"
                      : sharesShell(agent)
                        ? toolLive("terminal")
                          ? "Terminal — the shell this session shares, still running"
                          : "Terminal — the shell this session shares"
                        : toolLive("terminal")
                          ? "Terminal — your shell, still running (this assistant can't run commands)"
                          : "Terminal — your shell (this assistant can't run commands)"
                  }
                  data-testid="topbar-terminal"
                >
                  <Icon name="code" size={ICON.md} />
                  {/* The shell is still running behind a tab you are not on. Same
                      dot, same meaning as Artifacts: there is something in here. */}
                  {toolLive("terminal") && !toolOpen("terminal") && (
                    <span className="topbar-dot" aria-hidden />
                  )}
                </button>
              </>
            )}</>
  );
}
