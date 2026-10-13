import { useCallback, useEffect, useState } from "react";
import { getConnectors, getSessionConnections, type SessionConnections } from "../../api";
import { indexConnectors, type ConnectorMap } from "../../connectors/visuals";

/** This session's connections, and the connector index behind brand colors and links. */
export function useSessionConnections(sessionId: string, personaId: string | undefined, active: boolean) {
  const [conns, setConns] = useState<SessionConnections | null>(null);
  const [byName, setByName] = useState<ConnectorMap>({});
  const reload = useCallback(() => {
    // personaId hint: a brand-new session has no server-side record yet, so without it the
    // view would resolve to the DEFAULT persona's defaults/recommends.
    getSessionConnections(sessionId, personaId)
      .then(setConns)
      .catch(() => setConns(null));
  }, [sessionId, personaId]);
  useEffect(() => {
    reload();
  }, [reload]);

  // The connector index feeds brand colors and gates the "Channels ·" links; refetch every
  // time the tab is shown so a single failed fetch at mount can't hide them for the
  // session's whole lifetime.
  useEffect(() => {
    let live = true;
    getConnectors()
      .then((list) => live && setByName(indexConnectors(list)))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [active]);

  return { conns, byName, reload };
}
