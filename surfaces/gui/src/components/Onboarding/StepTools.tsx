import { useEffect, useState } from "react";
import { AVATAR } from "../Icon";
import { cloudLogin, connectManaged, getCloudStatus, getConnectors, type CloudStatus, type Connector } from "../../api";
import { ConnectorBadge } from "../../connectors/ConnectorIcon";
import { Spinner } from "../AutomationQuickstart";
import { TOOLS_SOON, TOOL_ROWS } from "./TOOL_ROWS";

/** Step 2. §41 (owner design, 2026-07-19, supersedes §39's card gallery): BENEFIT ROWS are
 * the connect surface — one row set, two states, ZERO layout shift. Pre-sign-in the rows
 * make the case and a pinned band asks for sign-in; after sign-in the band's slot keeps its
 * place but flips to a green congrats, and every row grows a quiet Connect pill. The gated
 * Google pair is ONE combined grayed row. */
export function StepTools({ onNext }: { onNext: () => void }) {
  // -- step 2: connect your everyday tools (§39 two-state page) -------------------
  const [connectors, setConnectors] = useState<Connector[]>([]);
  const [cloud, setCloud] = useState<CloudStatus | null>(null);
  const [signinPhase, setSigninPhase] = useState<"opening" | "waiting" | null>(null);
  // One in-flight connect at a time; clicking another card quietly resets the first.
  const [pendingTool, setPendingTool] = useState<string | null>(null);

  // Poll while on the tools page: sign-in AND vendor consents land out-of-band in
  // the system browser. Tighten while either is actually in flight.
  useEffect(() => {
    const load = () => {
      getConnectors().then(setConnectors).catch(() => {});
      getCloudStatus().then(setCloud).catch(() => {});
    };
    load();
    const fast = signinPhase === "waiting" || pendingTool !== null;
    const t = setInterval(load, fast ? 750 : 3000);
    return () => clearInterval(t);
  }, [signinPhase, pendingTool]);

  // The poll flips the card to ✓ when the consent lands.
  useEffect(() => {
    if (pendingTool && connectors.find((c) => c.name === pendingTool)?.connected)
      setPendingTool(null);
  }, [connectors, pendingTool]);

  const startTool = async (name: string) => {
    setPendingTool(name); // replaces any previous pending connect
    const res = await connectManaged(
      name,
      name === "hubspot" ? { access: "read" } : undefined, // least privilege in onboarding
    ).catch(() => ({ ok: false }));
    if (!res.ok) setPendingTool((cur) => (cur === name ? null : cur)); // silent reset — no error walls here
  };

  return (
    <section data-testid="ob-step-tools" className="flex-1 min-h-0 flex flex-col">
      <h1 className="text-xl font-semibold">Connect your everyday tools</h1>
      <p className="text-md text-muted mt-0.5 mb-3">
        Chat can only advise. Connected, your assistant does the actual work:
      </p>

      <div className="flex-1 min-h-0 overflow-y-auto pr-1" data-testid="ob-tool-gallery">
        {TOOL_ROWS.map(({ name, benefit, detail }) => {
          const c = connectors.find((x) => x.name === name);
          if (!c) return null;
          return (
            <div
              key={name}
              className="flex items-center gap-3 py-2 border-b border-paper last:border-0"
              data-testid={`ob-tool-${name}`}
            >
              <ConnectorBadge connector={c} size={AVATAR.lg} title={c.title} />
              <span className="min-w-0 flex-1">
                <span className="block text-md font-semibold leading-tight">{benefit}</span>
                <span className="block text-sm text-muted truncate">{detail}</span>
              </span>
              {cloud?.signed_in &&
                (c.connected ? (
                  <span className="text-sm text-ok font-medium shrink-0">✓ Connected</span>
                ) : pendingTool === name ? (
                  <span className="text-sm text-muted shrink-0">Check your browser…</span>
                ) : (
                  <button
                    className="shrink-0 rounded-full border border-line px-4 py-1.5 text-sm font-medium hover:border-lineStrong"
                    onClick={() => startTool(name)}
                  >
                    Connect
                  </button>
                ))}
            </div>
          );
        })}
        {/* The gated Google pair: one combined grayed row, both states (§41). */}
        <div className="flex items-center gap-3 py-2" data-testid="ob-tool-google-soon">
          <span className="flex gap-1.5 opacity-40 grayscale">
            {TOOLS_SOON.map((n) => {
              const c = connectors.find((x) => x.name === n);
              return c ? <ConnectorBadge key={n} connector={c} size={AVATAR.md} title={c.title} /> : null;
            })}
          </span>
          <span className="min-w-0 flex-1">
            <span className="block text-md font-semibold leading-tight text-faint">
              Gmail &amp; Google Calendar
            </span>
            <span className="block text-sm text-faint truncate">
              Coming soon — pending Google&rsquo;s app verification.
            </span>
          </span>
          {cloud?.signed_in && <span className="text-xs text-faint shrink-0">Coming soon</span>}
        </div>
      </div>

      {/* The band is PINNED outside the scroll area and its slot never moves: the ask
          pre-sign-in, a green congrats after — zero layout shift at the moment the user
          returns from the browser (§41). */}
      {!cloud?.signed_in ? (
        <div className="mt-3.5 rounded-xl border border-line bg-paper px-4 py-3 flex items-center gap-3.5 shrink-0">
          <span className="flex-1 text-sm text-muted leading-snug">
            <span className="block text-md font-semibold text-ink mb-0.5">
              Sign in for one-click connections
            </span>
            Superuser handles the OAuth for 20+ tools — no dev consoles, no pasted keys.
            Tokens stay on this computer.
          </span>
          {signinPhase ? (
            <span className="inline-flex items-center gap-2 text-sm text-muted shrink-0">
              <Spinner />
              {signinPhase === "opening" ? (
                "Opening browser…"
              ) : (
                <>
                  Waiting…{" "}
                  <button
                    className="underline hover:text-ink"
                    onClick={() => setSigninPhase(null)}
                    data-testid="ob-signin-cancel"
                  >
                    Cancel
                  </button>
                </>
              )}
            </span>
          ) : (
            <button
              className="shrink-0 px-5 py-2 rounded-full bg-ink text-panel text-md"
              onClick={async () => {
                setSigninPhase("opening");
                await cloudLogin().catch(() => {});
                setSigninPhase("waiting");
              }}
              data-testid="ob-cloud-signin"
            >
              Sign in
            </button>
          )}
        </div>
      ) : (
        <div
          className="mt-3.5 rounded-xl border border-line bg-okSoft px-4 py-3 shrink-0"
          data-testid="ob-tools-signedin"
        >
          <span className="block text-md font-semibold text-ok mb-0.5">
            🎉 You&rsquo;re signed in{cloud.account ? ` as ${cloud.account}` : ""}
          </span>
          <span className="block text-sm text-muted">
            Connect a tool above with one click — or add them anytime later from the
            Connectors page.
          </span>
        </div>
      )}

      {/* One footer button, one slot: quiet skip pre-sign-in, black Next after. */}
      <div className="flex items-center mt-3.5">
        {cloud?.signed_in ? (
          <button
            className="ml-auto px-6 py-2 rounded-full bg-ink text-panel text-md shrink-0"
            onClick={onNext}
            data-testid="ob-continue-tools"
          >
            Next
          </button>
        ) : (
          <button
            className="ml-auto px-5 py-2 rounded-full border border-line text-md text-muted hover:text-ink hover:border-lineStrong shrink-0"
            onClick={onNext}
            data-testid="ob-tools-skip"
          >
            Continue without sign-in
          </button>
        )}
      </div>
      <p className="text-xs text-faint mt-3">
        30+ more tools on the Connectors page — add or remove anytime. Tokens stay on
        this computer.
      </p>
    </section>
  );
}
