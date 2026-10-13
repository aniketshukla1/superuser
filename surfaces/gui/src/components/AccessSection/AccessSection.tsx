import { useEffect, useRef, useState } from "react";
import { setSessionConnection, setSessionConnectionScope, type ConnectionScope, type Connector } from "../../api";
import { labelFor } from "../../connectors/visuals";
import { useRoots } from "../../useRoots";
import { SEC_H } from "../../ui";
import { ChannelsInline, ConnectInline } from "./ChannelsInline";
import { AddSourcePicker } from "./AddSourcePicker";
import { FoldersBlock } from "./FoldersBlock";
import { RecommendedList } from "./RecommendedList";
import { SourceRow } from "./SourceRow";
import { useChannelSubscriptions } from "./useChannelSubscriptions";
import { useCloudWhileConnecting } from "./useCloudWhileConnecting";
import { useSessionConnections } from "./useSessionConnections";

export function AccessSection({
  sessionId,
  personaId,
  branch,
  scratchPrimary,
  active = true,
  openKey = 0,
  onOpenIntegrations,
}: {
  sessionId: string;
  personaId?: string;
  branch?: string | null;
  scratchPrimary?: boolean;
  /** The Access tab is the visible one. The panel keeps every tab mounted, so without
   *  this the section would load once and then never notice a connector appearing. */
  active?: boolean;
  // Bumped by deep links ("Configure ›", onboarding's Start-working) → expand + scroll here.
  openKey?: number;
  onOpenIntegrations?: () => void;
}) {
  const { roots, busy: rootsBusy, error: rootsError, addRoot, toggleAccess, removeRoot } =
    useRoots(sessionId, active ? 1 : 0);
  const rootEl = useRef<HTMLElement | null>(null);

  const { conns, byName, reload } = useSessionConnections(sessionId, personaId, active);

  // Deep link. Selecting the tab is the panel's job now; this only makes sure the
  // section is actually in view once it is (ignore the mount value).
  const seenKey = useRef(openKey);
  useEffect(() => {
    if (openKey === seenKey.current) return;
    seenKey.current = openKey;
    setTimeout(() => rootEl.current?.scrollIntoView({ block: "nearest" }), 30);
  }, [openKey]);

  // Child views (connect-in-context / channels drill-down) replace the section body inline.
  const [channelsFor, setChannelsFor] = useState<string | null>(null);
  const [connectFor, setConnectFor] = useState<Connector | null>(null);
  // "+ Add a source…" (§32 addendum): the FULL catalog in-session. The list shows on focus,
  // before any typing (FB-012: typing-to-see was a hidden step), and the query filters it
  // live; rich browsing (detail pages, connect states) stays on the global Connectors page.
  const [adding, setAdding] = useState(false);
  const [query, setQuery] = useState("");
  // The add flow guarantees the new source is live HERE: the user asked for it in this
  // session, so after the connect lands it is also enabled per-session explicitly.
  const [addedFrom, setAddedFrom] = useState<string | null>(null);
  // Folders mirrors Sources: flat rows + a quiet "+" link that expands the inline form.
  const [addingFolder, setAddingFolder] = useState(false);
  const cloud = useCloudWhileConnecting(!!connectFor);
  const { recent, draft, setDraft, addErr, setAddErr, channelsOf, addChannel, removeChannel } = useChannelSubscriptions(sessionId, active, channelsFor);

  // Leaving the tab closes any child view — coming back starts at the top level.
  useEffect(() => {
    if (!active) {
      setChannelsFor(null);
      setConnectFor(null);
      setAdding(false);
      setQuery("");
      setAddedFrom(null);
      setAddingFolder(false);
    }
  }, [active]);

  const toggleSession = async (connector: string, next: boolean) => {
    await setSessionConnection(sessionId, connector, next);
    reload();
  };
  const pickScope = async (connector: string, next: ConnectionScope) => {
    await setSessionConnectionScope(sessionId, connector, next);
    reload();
  };

  const connected = conns?.connected ?? [];
  const recommended = conns?.recommended ?? [];

  return (
    <section ref={rootEl} data-testid="access-section">
      <div role="region" aria-label="Session access">
          {connectFor ? (
            <ConnectInline
              c={connectFor}
              cloud={cloud}
              onDone={() => {
                const name = connectFor.name;
                setConnectFor(null);
                if (addedFrom === name) {
                  // Added from THIS session's panel → also enable it here explicitly (a
                  // catalog connector need not be in the persona's default-on set).
                  setAddedFrom(null);
                  setSessionConnection(sessionId, name, true)
                    .catch(() => {})
                    .finally(reload);
                  return;
                }
                reload();
              }}
              onBack={() => {
                setConnectFor(null);
                setAddedFrom(null);
              }}
            />
          ) : channelsFor ? (
            <ChannelsInline
              label={labelFor(channelsFor, byName)}
              channels={channelsOf(channelsFor)}
              recent={recent}
              draft={draft}
              onDraft={(v) => {
                setDraft(v);
                setAddErr(null);
              }}
              onAdd={addChannel}
              error={addErr}
              onRemove={removeChannel}
              onBack={() => setChannelsFor(null)}
            />
          ) : (
            <div className="space-y-4">
              {/* Sources — each toggle is a per-session override (mute for THIS session only). */}
              <div>
                <div className={`${SEC_H} mb-1.5`}>Sources</div>
                {connected.length === 0 && (
                  <div className="text-sm text-faint py-0.5">
                    No connectors enabled for this session.
                  </div>
                )}
                <div className="space-y-1">
                  {connected.map((c) => (
                    <SourceRow
                      key={c.connector}
                      c={c}
                      byName={byName}
                      channelCount={channelsOf(c.connector).length}
                      onChannels={() => {
                        setDraft("");
                        setChannelsFor(c.connector);
                      }}
                      onScope={(next) => pickScope(c.connector, next)}
                      onToggle={(next) => toggleSession(c.connector, next)}
                    />
                  ))}
                </div>
                {connected.length > 0 && (
                  <p className="text-xs text-faint mt-1 leading-snug">
                    Off mutes it for <b>this session only</b> — the connector stays connected.
                    <br />
                    <b>Read</b> lets this session consult a source but never send through it.
                  </p>
                )}
                <AddSourcePicker
                  adding={adding}
                  setAdding={setAdding}
                  query={query}
                  setQuery={setQuery}
                  byName={byName}
                  connected={connected}
                  onPick={(c) => {
                    setAddedFrom(c.name);
                    setConnectFor(c);
                  }}
                />
                {/* Lives with its list (tester ask 2026-07-26): each group's manage link sits
                    directly under that group, not pooled at the section's bottom. */}
                <button
                  className="mt-1.5 block text-sm text-accent font-medium hover:underline text-left"
                  onClick={() => onOpenIntegrations?.()}
                >
                  Manage all connectors (global) →
                </button>
              </div>

              <RecommendedList
                recommended={recommended}
                byName={byName}
                onConnect={(connector) => {
                  // Connect IN CONTEXT when we ship this connector; unknown refs
                  // (no descriptor) still fall back to the global page.
                  const desc = byName[connector];
                  if (desc) setConnectFor(desc);
                  else onOpenIntegrations?.();
                }}
              />

              <FoldersBlock
                roots={roots}
                busy={rootsBusy}
                error={rootsError}
                addRoot={addRoot}
                toggleAccess={toggleAccess}
                removeRoot={removeRoot}
                scratchPrimary={scratchPrimary}
                branch={branch}
                addingFolder={addingFolder}
                setAddingFolder={setAddingFolder}
              />
            </div>
          )}
      </div>
    </section>
  );
}
