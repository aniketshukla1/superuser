import { useState } from "react";
import { connectManaged, type CloudStatus, type Connector } from "../../../api";
import { CloudSignInInline, CloudStatusPending } from "../CloudSignIn";
import { PILL_ACCENT, TAG_ACCENT } from "../ui";

// One-click pane for generic managed connectors (Notion, Attio, …): sign in
// with the service in the browser; each consent lands as its own account.
export function GenericOneClick({ c, cloud }: { c: Connector; cloud: CloudStatus | null }) {
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const go = async () => {
    setError(null);
    const res = await connectManaged(c.name);
    if (res.ok) setWaiting(true);
    else setError(res.error || "could not start the connect");
  };
  return (
    <div className="px-5 py-4 space-y-3">
      <p className="text-md text-muted">
        Opens {c.title} in your browser — approve access there. No tokens typed; connect
        again with another account to add it alongside.
      </p>
      {cloud?.signed_in ? (
        <button
          className={PILL_ACCENT + " w-full !py-2"}
          data-testid="modal-generic-one-click"
          onClick={go}
          disabled={waiting}
        >
          {waiting ? "Check your browser…" : `Connect ${c.title}`}
        </button>
      ) : cloud ? (
        <CloudSignInInline />
      ) : (
        <CloudStatusPending />
      )}
      {error && <div className="text-sm text-danger">{error}</div>}
      <p className="text-sm text-faint text-center flex items-center justify-center gap-1.5">
        <span className={TAG_ACCENT}>Recommended</span> tokens stay on this computer
      </p>
    </div>
  );
}

export function SlackOneClick({ c, cloud }: { c: Connector; cloud: CloudStatus | null }) {
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const go = async () => {
    setError(null);
    const res = await connectManaged(c.name);
    if (res.ok) setWaiting(true);
    else setError(res.error || "could not start the install");
  };
  return (
    <div className="px-5 py-4 space-y-3">
      <p className="text-md text-muted">
        Opens Slack in your browser — approve @ocw for the workspace. No tokens; works for any
        number of workspaces.
      </p>
      {cloud?.signed_in ? (
        <button className={PILL_ACCENT + " w-full !py-2"} data-testid="modal-add-to-slack" onClick={go} disabled={waiting}>
          {waiting ? "Check your browser…" : "Add to Slack"}
        </button>
      ) : cloud ? (
        <CloudSignInInline />
      ) : (
        <CloudStatusPending />
      )}
      {error && <div className="text-sm text-danger">{error}</div>}
      <p className="text-sm text-faint text-center flex items-center justify-center gap-1.5">
        <span className={TAG_ACCENT}>Recommended</span> relay · tokens stay on this computer
      </p>
    </div>
  );
}

export function GithubOneClick({ c, cloud }: { c: Connector; cloud: CloudStatus | null }) {
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const go = async () => {
    setError(null);
    const res = await connectManaged(c.name);
    if (res.ok) setWaiting(true);
    else setError(res.error || "could not start the install");
  };
  return (
    <div className="px-5 py-4 space-y-3">
      <p className="text-md text-muted">
        Opens GitHub in your browser — approve Superuser there. An existing @ocw-agent App
        installation links right up; otherwise you'll pick an account and repos. No tokens
        typed; the agent acts as ocw-agent[bot].
      </p>
      {cloud?.signed_in ? (
        /* One button: the broker is authorize-first — it links an existing installation or
           redirects the same tab on to the install page (the old "Already installed? Link
           it" question and the Configure dead-end are gone). */
        <button className={PILL_ACCENT + " w-full !py-2"} data-testid="modal-install-github-app" onClick={() => go()} disabled={waiting}>
          {waiting ? "Check your browser…" : "Connect GitHub"}
        </button>
      ) : cloud ? (
        <CloudSignInInline />
      ) : (
        <CloudStatusPending />
      )}
      {error && <div className="text-sm text-danger">{error}</div>}
      <p className="text-sm text-faint text-center flex items-center justify-center gap-1.5">
        <span className={TAG_ACCENT}>Recommended</span> relay · short-lived tokens, never stored
      </p>
    </div>
  );
}

export function HubSpotOneClick({ c, cloud }: { c: Connector; cloud: CloudStatus | null }) {
  const [access, setAccess] = useState<"read" | "write">("read");
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const go = async () => {
    setError(null);
    const res = await connectManaged(c.name, { access });
    if (res.ok) setWaiting(true);
    else setError(res.error || "could not start the connect");
  };
  return (
    <div className="px-5 py-4 space-y-3">
      <p className="text-md text-muted">
        Opens HubSpot in your browser — pick the portal there. What agents may do is chosen
        NOW, at consent:
      </p>
      <div className="space-y-1.5" data-testid="hubspot-access">
        {(
          [
            ["read", "Read-only", "search and read contacts, companies, deals, tickets"],
            ["write", "Read & write", "adds: log notes and tasks, update records, create contacts — never delete"],
          ] as const
        ).map(([value, label, blurb]) => (
          <label key={value} className="flex items-start gap-2 text-md cursor-pointer">
            <input
              type="radio"
              name="hubspot-access"
              className="mt-0.5"
              checked={access === value}
              data-testid={`hubspot-access-${value}`}
              onChange={() => setAccess(value)}
            />
            <span>
              <span className="font-medium">{label}</span>
              <span className="block text-sm text-muted">{blurb}</span>
            </span>
          </label>
        ))}
      </div>
      {cloud?.signed_in ? (
        <button className={PILL_ACCENT + " w-full !py-2"} data-testid="modal-connect-hubspot" onClick={go} disabled={waiting}>
          {waiting ? "Check your browser…" : "Connect HubSpot"}
        </button>
      ) : cloud ? (
        <CloudSignInInline />
      ) : (
        <CloudStatusPending />
      )}
      {error && <div className="text-sm text-danger">{error}</div>}
      <p className="text-sm text-faint text-center">
        Works for any number of portals · tokens stay on this computer
      </p>
    </div>
  );
}
