import { isTauri, type DevServer } from "../../tauri";
import { Icon, ICON } from "../Icon";

/** What the Browser tab shows before a page is open: the project's dev servers (run,
 * stop, read their output), the folder they come from, and how to add more. */
export function BrowserStartPage({
  servers,
  running,
  starting,
  logs,
  openLog,
  setOpenLog,
  project,
  pickProject,
  runServer,
  stopServer,
}: {
  servers: DevServer[];
  running: string[];
  starting: string;
  logs: Record<string, string>;
  openLog: string;
  setOpenLog: (name: string) => void;
  project: string;
  pickProject: () => void;
  runServer: (s: DevServer) => void;
  stopServer: (name: string) => void;
}) {
  return (
    <div className="browser-start" data-testid="browser-start">
      <p className="browser-start-lede">
        {servers.length
          ? "Run a server below to see your app, or enter a URL above to browse."
          : "Enter a URL above to browse."}
      </p>

      {servers.length > 0 && (
        <div className="browser-servers">
          {servers.map((s) => {
            const isUp = running.includes(s.name);
            const log = logs[s.name];
            return (
              <div className="browser-server-wrap" key={s.name}>
                <div className="browser-server">
                  <Icon name="code" size={ICON.md} />
                  <span className="browser-server-name" title={`${s.command}\nin ${project}`}>
                    {s.name}
                  </span>
                  {s.port && <span className="browser-server-port">:{s.port}</span>}
                  {log && (
                    <button
                      className="browser-server-log"
                      onClick={() => setOpenLog(openLog === s.name ? "" : s.name)}
                      title="Show what this server printed"
                    >
                      Log
                    </button>
                  )}
                  {isUp || starting === s.name ? (
                    <button
                      className="browser-server-run stop"
                      onClick={() => stopServer(s.name)}
                      title={`Stop ${s.name}`}
                      aria-label={`Stop ${s.name}`}
                    >
                      {starting === s.name ? "Starting…" : "Stop"}
                    </button>
                  ) : (
                    <button
                      className="browser-server-run"
                      onClick={() => runServer(s)}
                      title={`Run in ${project}: ${s.command}`}
                      aria-label={`Run ${s.name}`}
                    >
                      <Icon name="chevronRight" size={ICON.md} />
                    </button>
                  )}
                </div>
                {openLog === s.name && log && <pre className="browser-server-output">{log.trimEnd()}</pre>}
              </div>
            );
          })}
        </div>
      )}

      {/* Always say which folder this read, and always let it be changed — the
          project you want to run isn't always the one the conversation is about. */}
      <div className="browser-project">
        {project ? (
          <>
            <Icon name="folder" size={ICON.md} />
            <span className="browser-project-path" title={project}>
              {project.replace(/^\/Users\/[^/]+/, "~")}
            </span>
            <button className="browser-project-change" onClick={pickProject}>
              Change
            </button>
          </>
        ) : (
          <button className="browser-server-run" onClick={pickProject}>
            Choose a project folder
          </button>
        )}
      </div>

      <p className="browser-start-hint">
        {!project
          ? "Pick a folder with a .claude/launch.json to run its dev servers."
          : servers.length
            ? "Edit servers in .claude/launch.json."
            : "No servers here — add them in .claude/launch.json."}
      </p>

      {!isTauri() && (
        <p className="browser-start-hint">
          Browsing needs the desktop app — it runs a real webview, which the browser build has
          no way to embed.
        </p>
      )}
    </div>
  );
}
