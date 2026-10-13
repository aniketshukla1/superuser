import { useEffect, useState } from "react";
import { getTrustedWorkspaces, setWorkspaceTrusted, type WorkspaceCommandTrust } from "../../api";
import { checkForUpdate, installUpdate } from "../../tauri";
import { reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";
import { BTN_BORDERED, INPUT } from "./SettingsView";

export function TrustedWorkspacesCard() {
  const [workspaces, setWorkspaces] = useState<WorkspaceCommandTrust[] | null>(null);
  // The card listed what was trusted and offered no way to trust anything: the only route in
  // was a prompt that appears while a project is asking for something.
  const [adding, setAdding] = useState("");
  // Revoking asks in place, the way deleting anything else in this app does, rather than in a
  // system dialog that lands in the corner of the screen.
  const [confirmRevoke, setConfirmRevoke] = useState<string | null>(null);

  const refresh = () =>
    getTrustedWorkspaces()
      .then(setWorkspaces)
      .catch(() => setWorkspaces([]));

  useEffect(() => {
    refresh();
  }, []);

  const trust = async () => {
    const path = adding.trim();
    if (!path) return;
    setAdding("");
    await reportIfFailed(setWorkspaceTrusted(path, true), `Couldn't trust ${path}.`);
    refresh();
  };

  const revoke = async (path: string) => {
    setConfirmRevoke(null);
    await reportIfFailed(setWorkspaceTrusted(path, false), `Couldn't revoke command trust for ${path}.`);
    refresh();
  };

  return (
    <div className={CARD + " p-4 mb-4"} data-testid="trusted-workspaces-card">
      <div className={FIELD_LABEL}>Trusted workspaces</div>
      <div className={FIELD_HELP}>
        Trusted projects may declare command allowances in .superuser/config.json. Until you trust a project, what it declares does nothing.
      </div>
      {workspaces === null ? (
        <div className="text-sm text-muted mt-3">Loading…</div>
      ) : workspaces.length === 0 ? (
        <div className="text-sm text-muted mt-3">No workspaces are trusted.</div>
      ) : (
        <div className="mt-3 divide-y divide-line">
          {workspaces.map((workspace) => (
            <div key={workspace.workspace} className="py-2.5 flex items-start gap-3">
              <div className="min-w-0 flex-1">
                <div className="text-sm text-ink break-all">{workspace.workspace}</div>
                <div className="text-xs text-muted mt-0.5">
                  {workspace.requested_commands.length
                    ? `${workspace.requested_commands.length} project command allowance${workspace.requested_commands.length === 1 ? "" : "s"}`
                    : "No project command allowances currently declared"}
                  {!workspace.exists ? " · Folder unavailable" : ""}
                </div>
              </div>
              {confirmRevoke === workspace.workspace ? (
                <span className="flex items-center gap-2 shrink-0">
                  <button
                    className="btn sm danger"
                    data-testid="trust-revoke-confirm"
                    onClick={() => void revoke(workspace.workspace)}
                  >
                    Revoke
                  </button>
                  <button className="btn sm ghost" onClick={() => setConfirmRevoke(null)}>
                    Keep
                  </button>
                </span>
              ) : (
                <button
                  className="btn sm shrink-0"
                  onClick={() => setConfirmRevoke(workspace.workspace)}
                >
                  Revoke…
                </button>
              )}
            </div>
          ))}
        </div>
      )}
      <div className="mt-3 flex items-center gap-2">
        <input
          className={INPUT + " flex-1 min-w-0"}
          type="text"
          aria-label="Folder to trust"
          placeholder="/Users/you/code/project"
          spellCheck={false}
          value={adding}
          data-testid="trust-path"
          onChange={(e) => setAdding(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && void trust()}
        />
        <button className="btn shrink-0" disabled={!adding.trim()} onClick={() => void trust()}>
          Trust this folder
        </button>
      </div>
    </div>
  );
}

export function UpdateInline() {
  const [state, setState] = useState<"idle" | "checking" | "none" | "found" | "installing" | "error">("idle");
  const [version, setVersion] = useState("");
  // Why there is nothing, when there is nothing.
  const [updateNote, setUpdateNote] = useState<string | null>(null);

  const check = async () => {
    setState("checking");
    try {
      const u = await checkForUpdate();
      if (u?.available && u.version) {
        setVersion(u.version);
        setState("found");
      } else {
        // The reason, when there is one, is the difference between "you are up to
        // date" and "this build never checks" — which used to look identical.
        setUpdateNote(u?.reason ?? null);
        setState("none");
      }
    } catch {
      setState("error");
    }
  };

  const install = async () => {
    setState("installing");
    try {
      await installUpdate(); // success restarts the app
    } catch {
      setState("error");
    }
  };

  return (
    <span className="inline-flex items-center gap-2.5">
      {state === "found" ? (
        <button className={BTN_BORDERED} onClick={install} data-testid="settings-update-install">
          Update to v{version} and restart
        </button>
      ) : (
        <button
          className={BTN_BORDERED}
          onClick={check}
          disabled={state === "checking" || state === "installing"}
          data-testid="settings-update-check"
        >
          {state === "checking" ? "Checking…" : "Check for updates"}
        </button>
      )}
      {(state === "none" || state === "error" || state === "installing") && (
        <span className="text-sm text-muted">
          {state === "none"
            ? // Saying "you're on the latest version" when nothing was actually
              // checked is the lie this whole command used to tell by returning
              // null. If there is a reason, it is the truer answer.
              (updateNote ?? "You're on the latest version.")
            : state === "error"
              ? "Couldn't check right now — try again later."
              : "Downloading — Superuser restarts by itself when it's ready."}
        </span>
      )}
    </span>
  );
}
