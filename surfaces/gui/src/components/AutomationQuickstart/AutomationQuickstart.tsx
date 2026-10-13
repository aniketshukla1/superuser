import { useEffect, useRef, useState } from "react";
import { TEMPLATES, cronFor, type QuickTemplate } from "./TEMPLATES";
import { CloudSignInPane } from "./CloudSignInPane";
import { ConnectionRows } from "./ConnectionRows";
import { RecipeFields } from "./RecipeFields";
import { TemplateGrid } from "./TemplateGrid";
import { useQuickstartConnections } from "./useQuickstartConnections";

export function AutomationQuickstart({
  busy,
  onCreate,
}: {
  busy: boolean;
  onCreate: (payload: {
    title: string;
    instructions: string;
    cron?: string;
    permissions?: { tool: string; target: string; access: "read" | "write" }[];
  }) => void;
}) {
  const [pickedKey, setPickedKey] = useState<string | null>(null);
  const picked = TEMPLATES.find((t) => t.key === pickedKey) || null;

  const { connState, allConnected, cloud, recent, pendingConn, connFlow, setConnFlow, signinPhase, startConnect, cancelSignin, signInThenConnect } =
    useQuickstartConnections(pickedKey, picked);
  const [repo, setRepo] = useState("");
  const [channel, setChannel] = useState("");
  const [day, setDay] = useState("mon");
  const [time, setTime] = useState("09:00");
  const [deliver, setDeliver] = useState<"app" | "slack">("app");
  const [consent, setConsent] = useState(true);


  // §25 consent line shows the HUMAN name (owner catch 2026-07-14: it echoed the raw
  // slack:T…/C… target). Names come from a picker pick (remembered per address) or the
  // recent list; a hand-typed raw address stays raw — we never guess.
  const [picked_names, setPickedNames] = useState<Record<string, { name: string; workspace?: string }>>({});
  const pickedInfo = picked_names[channel];
  const channelName = pickedInfo?.name || recent.find((c) => c.channel === channel)?.name;
  const channelLabel = channelName ? `#${channelName}` : channel;
  const channelWorkspace = pickedInfo?.workspace;


  // §30: the configure card scrolls into view on pick — it expands below the fold on
  // three-row grids and otherwise appears "nowhere".
  const cfgRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (pickedKey) cfgRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }, [pickedKey]);

  const pick = (t: QuickTemplate) => {
    setPickedKey(t.key);
    setDay(t.day);
    setTime(t.time);
    setConsent(true);
    setConnFlow(null);
  };


  const create = () => {
    if (!picked) return;
    onCreate({
      title: picked.title,
      instructions: picked.instructions({ repo, channel, deliver }),
      cron: cronFor(day, time),
      permissions:
        picked.consent && consent && channel
          ? [{ tool: "send_message", target: channel, access: "write" }]
          : [],
    });
  };

  const gateHint = !allConnected
    ? `Connect ${picked?.conns
        .filter((c) => !connState(c.name)?.connected)
        .map((c) => connState(c.name)?.title || c.name)
        .join(" and ")} to continue`
    : picked?.needsChannel && !channel
      ? "Pick a channel to post to first"
      : "";


  return (
    <div className="mb-4">
      <div className="text-xs text-faint mb-2.5">
        Start from a template
      </div>
      <TemplateGrid pickedKey={pickedKey} connState={connState} onPick={pick} />

      {picked && (
        <div
          ref={cfgRef}
          className="mt-3 rounded-xl2 border border-line bg-panel p-4"
          data-testid="qs-configure"
        >
          {/* §30: the card names its template — without this it starts abruptly after the grid. */}
          <div className="flex items-baseline gap-2 pb-2.5 mb-1 border-b border-line">
            <span className="text-xs text-accent font-semibold">
              Set up
            </span>
            <span className="text-lg font-semibold">{picked.title}</span>
            <span className="ml-auto text-sm text-faint max-sm:hidden">
              {picked.conns.length ? "Connections, delivery & schedule" : "Delivery & schedule"} ·{" "}
              {picked.cadence}
            </span>
          </div>
          <ConnectionRows conns={picked.conns} connState={connState} connFlow={connFlow} setConnFlow={setConnFlow} startConnect={startConnect} />

          {pendingConn && !cloud?.signed_in && (
            <CloudSignInPane signinPhase={signinPhase} cancelSignin={cancelSignin} signInThenConnect={signInThenConnect} />
          )}

          {allConnected && (
            <RecipeFields
              picked={picked}
              repo={repo}
              setRepo={setRepo}
              channel={channel}
              setChannel={setChannel}
              recent={recent}
              onPickName={(address, name, workspace) =>
                setPickedNames((m) => ({ ...m, [address]: { name, workspace } }))
              }
              day={day}
              setDay={setDay}
              time={time}
              setTime={setTime}
              deliver={deliver}
              setDeliver={setDeliver}
              consent={consent}
              setConsent={setConsent}
              channelLabel={channelLabel}
              channelWorkspace={channelWorkspace}
            />
          )}

          <div className="flex items-center gap-3 mt-4">
            <button
              className="text-sm text-faint hover:text-muted"
              onClick={() => setPickedKey(null)}
            >
              Cancel
            </button>
            {/* A silently-disabled primary reads as a bug — always name the missing piece. */}
            {gateHint && (
              <span className="ml-auto text-xs text-faint" data-testid="ob-create-hint">
                {gateHint}
              </span>
            )}
            <button
              className={
                (gateHint ? "" : "ml-auto ") +
                "px-5 py-2 rounded-full bg-ink text-panel text-md disabled:opacity-40"
              }
              disabled={busy || !allConnected || (picked.needsChannel && !channel)}
              onClick={create}
              data-testid="ob-create"
            >
              {busy ? "Creating…" : "Create automation"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
