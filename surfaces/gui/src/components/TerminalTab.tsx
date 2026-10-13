// The Terminal tab — xterm.js over the shared PTY in core/terminal.rs.
//
// The shell is shared with the agent: when it calls `run_shell`, the command is
// typed into THIS shell and its output scrolls past here. So this isn't a log of
// what the agent did, it's the agent doing it. You can also take the keyboard at
// any point; it's your login shell.
import { useCallback, useEffect, useRef, useState } from "react";
import "@xterm/xterm/css/xterm.css";
import { useTheme } from "../theme";
import {
  isTauri,
  onTerminalExit,
  onTerminalOutput,
  terminalClose,
  terminalOpen,
  terminalResize,
  terminalWrite,
} from "../tauri";

/** Pull the terminal's colours from the app's own CSS variables so it changes with
 *  the theme instead of being a black rectangle pasted into a light UI. */
function themeFromCss(): Record<string, string> {
  const css = getComputedStyle(document.documentElement);
  const v = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
  return {
    background: v("--panel", "#111318"),
    foreground: v("--ink", "#d8dee9"),
    cursor: v("--accent", "#7aa2f7"),
    selectionBackground: v("--selection", "rgba(122,162,247,0.3)"),
  };
}

export function TerminalTab({ cwd, active }: { cwd: string; active: boolean }) {
  const host = useRef<HTMLDivElement | null>(null);
  // Kept across re-renders so the session survives tab switches — the scrollback is
  // the record of what the agent did, and remounting would throw it away.
  const term = useRef<import("@xterm/xterm").Terminal | null>(null);
  const fit = useRef<import("@xterm/addon-fit").FitAddon | null>(null);
  const [error, setError] = useState("");
  const [exited, setExited] = useState(false);
  // Re-read the colours when the theme changes, so an open terminal follows it too.
  const theme = useTheme();
  useEffect(() => {
    if (term.current) term.current.options.theme = themeFromCss();
  }, [theme]);

  useEffect(() => {
    if (!host.current || term.current) return;
    let disposed = false;
    const unlisten: Array<() => void> = [];

    (async () => {
      const [{ Terminal }, { FitAddon }] = await Promise.all([
        import("@xterm/xterm"),
        import("@xterm/addon-fit"),
      ]);
      if (disposed || !host.current) return;

      const t = new Terminal({
        // The app's one monospace face, read from the stylesheet: xterm takes a string, so
        // this is the one place the token has to be resolved rather than referenced.
        fontFamily:
          getComputedStyle(document.documentElement).getPropertyValue('--mono').trim() ||
          'ui-monospace, SFMono-Regular, Menlo, monospace',
        fontSize: 12,
        cursorBlink: true,
        // The agent can produce a lot of output in one turn; keep enough to scroll back
        // through a whole build.
        scrollback: 10000,
        theme: themeFromCss(),
      });
      const f = new FitAddon();
      t.loadAddon(f);
      t.open(host.current);
      f.fit();
      term.current = t;
      fit.current = f;

      if (!isTauri()) {
        t.write("\r\n  The terminal needs the desktop app — it runs a real shell.\r\n");
        return;
      }

      // `disposed` is re-checked after each await: cleanup can run while these are
      // in flight, and a listener registered afterwards would never be torn down.
      const track = (u: () => void) => (disposed ? u() : unlisten.push(u));
      track(await onTerminalOutput((bytes) => t.write(bytes)));
      track(
        await onTerminalExit(() => {
          setExited(true);
          t.write("\r\n\x1b[2m[shell exited]\x1b[0m\r\n");
        }),
      );
      t.onData((d) => terminalWrite(d));
      t.onResize(({ cols, rows }) => terminalResize(cols, rows));

      try {
        await terminalOpen(cwd, t.cols, t.rows);
      } catch (e) {
        setError(String((e as Error)?.message || e));
      }
    })();

    return () => {
      disposed = true;
      unlisten.forEach((u) => u());
      term.current?.dispose();
      term.current = null;
      // Kill the shell with the panel: a stray login shell per session would pile up.
      if (isTauri()) terminalClose();
    };
    // cwd is captured at open time on purpose — re-rooting a live shell would mean
    // killing it, and killing it would discard the scrollback.
  }, []);

  // At most one refit per frame. A fit() re-lays-out the host, which can wake the
  // ResizeObserver that asked for it; coalescing means that answers itself within the
  // frame instead of becoming a fit-per-frame treadmill — and every fit clears the
  // renderer and SIGWINCHes the shell, so a treadmill is a visible flicker and a
  // prompt redrawn forever.
  const pending = useRef<number | null>(null);
  const scheduleFit = useCallback(() => {
    if (pending.current !== null) return;
    pending.current = requestAnimationFrame(() => {
      pending.current = null;
      // A display:none host measures zero; fitting to that resizes the shell to 1x1.
      if (!host.current?.clientWidth || !host.current.clientHeight) return;
      fit.current?.fit();
    });
  }, []);

  useEffect(
    () => () => {
      if (pending.current !== null) cancelAnimationFrame(pending.current);
    },
    [],
  );

  // The tab is display:none while inactive, so xterm measures zero and has to be
  // re-fit when it comes back — otherwise it renders one column wide.
  useEffect(() => {
    if (active) scheduleFit();
  }, [active, scheduleFit]);

  useEffect(() => {
    if (!host.current) return;
    const ro = new ResizeObserver(() => {
      if (active) scheduleFit();
    });
    ro.observe(host.current);
    return () => ro.disconnect();
  }, [active, scheduleFit]);

  return (
    <div className="terminal-tab">
      {error && <div className="rail-error terminal-error">{error}</div>}
      {exited && (
        <button className="btn sm terminal-restart" onClick={() => window.location.reload()}>
          Restart shell
        </button>
      )}
      <div className="terminal-host" ref={host} data-testid="terminal-host" />
    </div>
  );
}
