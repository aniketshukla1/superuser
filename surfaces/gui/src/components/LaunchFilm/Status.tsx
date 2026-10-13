/**
 * Under the name, only when the app cannot start: what went wrong and a way to try again.
 * Ready, and on its way, say nothing here (owner, 2026-10-13: the `root@localhost:~#
 * ready` line under the tagline went) — the bottom bar already reads out each step.
 */
export function Status({ failed, onRetry }: { failed?: boolean; onRetry?: () => void }) {
  if (!failed) return null;
  return (
    <div className="film-status film-trouble" data-testid="boot-trouble">
      <div className="film-trouble-title">Can’t reach the local runtime</div>
      <p className="film-explain">
        Superuser talks to a helper process on this machine, and it isn&rsquo;t answering on 127.0.0.1:8765. Nothing
        has been lost — your sessions and memory are on disk. Quitting and reopening the app usually restarts it.
      </p>
      <button className="film-retry" onClick={onRetry} data-testid="boot-retry">
        Try again
      </button>
    </div>
  );
}
