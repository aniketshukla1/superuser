import { useEffect, useState } from "react";
import { getBugRepro, setBugRepro, type BugReproSetup } from "../api";
import { CARD } from "../ui";

// Bug Reproduction's settings.
//
// Reproduces a bug report on staging in its own browser, which opens the address set here and
// nothing else — production, and the customers' data in it, stay out of reach. What its
// browser still needs to reach staging is said here, so a refusal later isn't the first word.

const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
const BTN = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";

/** What stands between the assistant and staging, first thing first; `null` when nothing does. */
function missing(r: BugReproSetup): string | null {
  if (!r.staging) return "No staging address yet, so its browser opens nothing.";
  if (!r.browser) return "Turn on the assistants' browser in Settings ▸ Browser.";
  if (r.on_this_mac && !r.loopback) return "Staging is on this Mac: allow this Mac's own dev servers in Settings ▸ Browser.";
  if (r.local_only && !r.on_this_mac)
    return "Local-only mode is on, so only this Mac's own servers are reachable. Turn it off to reach staging.";
  return null;
}

export function BugReproSection() {
  const [repro, setRepro] = useState<BugReproSetup | null>(null);
  const [staging, setStaging] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const load = () =>
    getBugRepro().then((r) => {
      setRepro(r);
      if (r) setStaging(r.staging);
    });
  useEffect(() => {
    load();
  }, []);

  const save = async () => {
    setBusy(true);
    setNotice(null);
    const res = await setBugRepro({ repro_staging: staging.trim() });
    setBusy(false);
    if (res.ok === false) {
      setNotice(res.error ?? "Couldn't save that.");
      return;
    }
    await load();
    setNotice("Saved.");
  };

  if (!repro) return null;
  const gap = missing(repro);

  return (
    <section className={`${CARD} p-4 mt-6`} data-testid="bug-repro">
      <div className="text-md font-medium">Bug Reproduction</div>
      <p className="text-sm text-muted mt-1 mb-3 leading-prose">
        Reproduces a bug report on your staging site in its own browser — exact steps, screenshots, what the
        console said, and a minimal test case. Its browser opens this address and nothing else, so production and
        your customers' data stay out of reach. A test account's password is typed into the page by you, never
        into the chat.
      </p>

      <label className="text-sm font-medium text-ink" htmlFor="repro-staging">
        Staging address
      </label>
      <input
        id="repro-staging"
        className={`${INPUT} mt-1 mb-1`}
        placeholder="https://staging.example.com"
        value={staging}
        onChange={(e) => setStaging(e.target.value)}
      />
      <div className="text-xs text-muted mb-3">
        Host and port both: staging on :3000 isn't the service on :8080. It can be on your company's network — this is
        the one private address its browser may reach.
      </div>

      <div className="text-xs text-muted mb-3" data-testid="repro-ready">
        {gap ?? "Ready — ask Bug Reproduction in the sidebar to reproduce a bug."}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <button className={BTN} disabled={busy} onClick={save}>
          {busy ? "Saving…" : "Save"}
        </button>
        {notice ? (
          <span className="text-xs text-muted" role="status">
            {notice}
          </span>
        ) : null}
      </div>
    </section>
  );
}
