import { Spinner } from "./TEMPLATES";

/** One sign-in unlocks every one-click connection: the pane that asks for it. */
export function CloudSignInPane({
  signinPhase,
  cancelSignin,
  signInThenConnect,
}: {
  signinPhase: "opening" | "waiting" | null;
  cancelSignin: () => void;
  signInThenConnect: () => void;
}) {
  return (
    <div
      className="bg-accentSoft/50 rounded-xl px-4 py-3 mt-3 text-sm text-muted"
      data-testid="ob-cloudpane"
    >
      <span className="block text-md text-ink font-medium">
        One sign-in unlocks every one-click connection
      </span>
      Connections are brokered by Superuser Cloud — your tokens stay on this computer.
      <div className="flex items-center gap-3 mt-2">
        {signinPhase ? (
          <>
            <span className="inline-flex items-center gap-2 text-sm">
              <Spinner />
              {signinPhase === "opening" ? "Opening browser…" : "Waiting for sign-in…"}
            </span>
            {signinPhase === "waiting" && (
              <span className="text-xs text-faint">
                Finish signing in in your browser — this page updates by itself.{" "}
                <button
                  className="underline hover:text-muted"
                  onClick={cancelSignin}
                  data-testid="ob-signin-cancel"
                >
                  Cancel
                </button>
              </span>
            )}
          </>
        ) : (
          <button
            className="px-3.5 py-1 rounded-full border border-line text-sm text-accent hover:bg-panel"
            onClick={signInThenConnect}
            data-testid="ob-cloud-signin"
          >
            Sign in to Superuser Cloud
          </button>
        )}
      </div>
    </div>
  );
}
