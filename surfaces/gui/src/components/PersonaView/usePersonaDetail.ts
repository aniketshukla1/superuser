import { useEffect, useState } from "react";
import { getConnectors, getPersonaDetail, getPersonas, getWorkerTypes, setPersonaConnection, setPersonaEnabled, updatePersona, type Persona, type PersonaDetail, type WorkerType } from "../../api";
import { indexConnectors, type ConnectorMap } from "../../connectors/visuals";

/** One persona's page state: its detail, the workers and assistants it could use, and the
 * optimistic changes the page makes to it. */
export function usePersonaDetail(personaId: string) {
  const [detail, setDetail] = useState<PersonaDetail | null>(null);
  const [workers, setWorkers] = useState<WorkerType[]>([]);
  // Everyone this one could be allowed to ask. Fetched once; the list is short and the
  // page already reads the registry for everything else on it.
  const [others, setOthers] = useState<Persona[]>([]);
  const [byName, setByName] = useState<ConnectorMap>({});
  const [error, setError] = useState<string | null>(null);

  // The worker menu, for assistants that delegate. Fetched here rather than in the
  // section so an assistant with no `subagent` capability never asks for it.
  useEffect(() => {
    if (!detail?.tools.includes("subagent")) return;
    getWorkerTypes()
      .then((w) => setWorkers(w.types))
      .catch(() => setWorkers([]));
  }, [detail?.tools]);

  // Optimistic, then re-read: the server is the one that decides whether a mode is
  // a mode, and a select that keeps a value the server rejected is a control that
  // lies about what the assistant will do.
  const saveMode = async (mode: string) => {
    if (!detail) return;
    const before = detail.default_permission_mode;
    setDetail({ ...detail, default_permission_mode: mode });
    const r = await updatePersona(personaId, { default_permission_mode: mode });
    if (!r.ok) {
      setDetail((d) => (d ? { ...d, default_permission_mode: before } : d));
      setError(r.error || "Couldn't change the mode.");
    }
  };

  // Who this assistant may hand work to. Optimistic like the rest of the page, and put
  // back if the server disagrees — a tick that survives a refusal is a claim about what
  // the assistant may do, and it would be false.
  const toggleAsk = async (id: string, on: boolean) => {
    if (!detail) return;
    const before = detail.asks ?? [];
    const next = on ? [...before, id] : before.filter((x) => x !== id);
    setDetail({ ...detail, asks: next });
    const r = await updatePersona(personaId, { asks: next });
    if (!r.ok) {
      setDetail((d) => (d ? { ...d, asks: before } : d));
      setError(r.error || "Couldn't change who it may ask.");
    }
  };

  // Its own browser. The grant is persona state, so it can be changed
  // here as well as in Settings ▸ Browser. A failure is said in the section: the page-level
  // error screen would take the whole assistant away to report one checkbox.
  const [browserError, setBrowserError] = useState<string | null>(null);
  const toggleBrowser = async (on: boolean) => {
    if (!detail) return;
    const before = detail.tools;
    const next = on ? [...before.filter((t) => t !== "browser"), "browser"] : before.filter((t) => t !== "browser");
    setDetail({ ...detail, tools: next });
    setBrowserError(null);
    const r = await updatePersona(personaId, { browser: on });
    if (!r.ok) {
      setDetail((d) => (d ? { ...d, tools: before } : d));
      setBrowserError(r.error || "Couldn't change its browser.");
    }
  };

  const toggleWorker = async (name: string, on: boolean) => {
    if (!detail) return;
    const before = detail.subagents ?? [];
    const next = on ? [...before, name] : before.filter((n) => n !== name);
    setDetail({ ...detail, subagents: next });
    const r = await updatePersona(personaId, { subagents: next });
    if (!r.ok) {
      setDetail((d) => (d ? { ...d, subagents: before } : d));
      setError(r.error || "Couldn't change the team.");
    }
  };

  useEffect(() => {
    let live = true;
    getPersonas()
      .then((p) => live && setOthers(p))
      .catch(() => live && setOthers([]));
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    let live = true;
    setDetail(null);
    setError(null);
    getPersonaDetail(personaId)
      .then((d) => live && setDetail(d))
      .catch(() => live && setError("Could not load this persona."));
    getConnectors()
      .then((list) => live && setByName(indexConnectors(list)))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [personaId]);

  const toggleEnabled = async (next: boolean) => {
    setDetail((d) => (d ? { ...d, enabled: next } : d)); // optimistic
    const r = await setPersonaEnabled(personaId, next);
    if (!r.ok) getPersonaDetail(personaId).then(setDetail).catch(() => {});
  };

  const toggleDefault = async (connector: string, next: boolean) => {
    const r = await setPersonaConnection(personaId, connector, next);
    if (r.default_connections) {
      setDetail((d) => (d ? { ...d, default_connections: r.default_connections! } : d));
    } else {
      getPersonaDetail(personaId).then(setDetail).catch(() => {});
    }
  };

  return { detail, setDetail, workers, others, byName, error, browserError, saveMode, toggleAsk, toggleBrowser, toggleWorker, toggleEnabled, toggleDefault };
}
