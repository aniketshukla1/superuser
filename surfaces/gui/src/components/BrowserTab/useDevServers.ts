import { useEffect, useState } from "react";
import { getRecentWorkspaces } from "../../api";
import { browserDevServers, chooseFolder, devserverRunning, devserverStart, devserverStop, onDevserverExit, onDevserverOutput, type DevServer } from "../../tauri";
import { PROJECT_KEY, storedProject, waitForServer } from "./waitForServer";

/** The dev servers the start page offers: which project, what it can run, what is
 * running, and what each one printed. */
export function useDevServers(workspace: string | undefined, go: (url: string) => void, setError: (e: string) => void) {
  const [servers, setServers] = useState<DevServer[]>([]);
  const [starting, setStarting] = useState("");
  const [running, setRunning] = useState<string[]>([]);
  /** Per-server output — the "hidden terminal" this panel runs things in. */
  const [logs, setLogs] = useState<Record<string, string>>({});
  const [openLog, setOpenLog] = useState("");

  // Which folder's dev servers to offer.
  //
  // The session's own workspace when it has one — but most personas aren't
  // project-scoped, so `workspace` is usually null and the start page had nothing to
  // say. Claude Code always has a cwd to fall back on; here the equivalents are the
  // last folder chosen for this panel, then the most recent workspace the app knows
  // about. And it stays changeable, because the project you want to run is not always
  // the one the conversation is about.
  const [project, setProject] = useState<string>(() => workspace || storedProject());

  useEffect(() => {
    if (workspace) setProject(workspace);
  }, [workspace]);

  useEffect(() => {
    if (project) return;
    getRecentWorkspaces()
      .then((list) => {
        const first = list?.[0]?.path;
        if (first) setProject(first);
      })
      .catch(() => {
        /* no recents — the picker below is the way in */
      });
  }, [project]);

  useEffect(() => {
    if (!project) {
      setServers([]);
      return;
    }
    browserDevServers(project)
      .then((s) => setServers(s ?? []))
      .catch(() => setServers([]));
  }, [project]);

  const pickProject = async () => {
    const picked = await chooseFolder();
    if (!picked) return;
    setProject(picked);
    try {
      localStorage.setItem(PROJECT_KEY, picked);
    } catch {
      /* best effort */
    }
  };

  // The hidden terminal's output. Capped per server — this is for reading a failure,
  // not for scrolling a week of logs.
  useEffect(() => {
    // `listen` is async, so the unsubscribe arrives AFTER this effect returns. A
    // cleanup that only walks the collected list therefore unsubscribes nothing when
    // it runs first — which StrictMode guarantees — and every remount leaves another
    // live listener behind. Symptom that gave it away: one server exiting logged
    // "[exited with code 0]" twice.
    let cancelled = false;
    const un: Array<() => void> = [];
    const track = (p: Promise<() => void>) =>
      p.then((u) => (cancelled ? u() : un.push(u))).catch(() => {});

    track(
      onDevserverOutput(({ name, data }) =>
        setLogs((l) => ({ ...l, [name]: ((l[name] || "") + data).slice(-20000) })),
      ),
    );
    track(
      onDevserverExit(({ name, code }) => {
        setRunning((r) => r.filter((n) => n !== name));
        setStarting((s) => (s === name ? "" : s));
        setLogs((l) => ({ ...l, [name]: (l[name] || "") + `\n[exited with code ${code}]\n` }));
        if (code !== 0) setOpenLog(name);
      }),
    );
    return () => {
      cancelled = true;
      un.forEach((u) => u());
    };
  }, []);

  // Servers survive a tab switch or a reload of this component, so ask what's up.
  useEffect(() => {
    devserverRunning()
      .then((r) => setRunning(r ?? []))
      .catch(() => setRunning([]));
  }, []);

  /**
   * Start the server in its own pty rooted at `project`, then load it once it answers.
   *
   * Not the shared terminal, and not a tab switch. Both were wrong: that shell is
   * rooted at the *session's* workspace, so a relative command ran against the wrong
   * project (`npm --prefix frontend …` → ENOENT), and being thrown into the Terminal
   * tab is the opposite of what you asked the browser for. The output lands in the
   * log pane below instead, where a failure is readable without going anywhere.
   */
  const runServer = async (s: DevServer) => {
    setError("");
    setStarting(s.name);
    setLogs((l) => ({ ...l, [s.name]: "" }));
    setOpenLog("");
    try {
      await devserverStart(s.name, project, s.command);
    } catch (e) {
      setStarting("");
      setError(String((e as Error)?.message || e));
      return;
    }
    setRunning((r) => (r.includes(s.name) ? r : [...r, s.name]));
    const up = await waitForServer(s.url);
    setStarting("");
    if (!up) {
      // Don't send them hunting — put the reason on screen.
      setError(`${s.name} didn't answer on ${s.url}.`);
      setOpenLog(s.name);
      return;
    }
    go(s.url);
  };

  const stopServer = (name: string) => {
    devserverStop(name);
    setRunning((r) => r.filter((n) => n !== name));
    if (starting === name) setStarting("");
  };

  return { project, pickProject, servers, starting, running, logs, openLog, setOpenLog, runServer, stopServer };
}
