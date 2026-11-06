import { fetch, httpBase, jsonBody, listOf } from "./neverRejectingJson";

// A session's folders: its scratch folder plus any the user added.

export interface RootInfo {
  label: string;
  path: string;
  primary: boolean;
  writable: boolean;
  exists: boolean;
}

type RootsChange = { ok: boolean; error?: string; roots?: RootInfo[] };

const rootsUrl = (sessionId: string) => `${httpBase()}/v1/sessions/${encodeURIComponent(sessionId)}/roots`;

export async function getRoots(sessionId: string): Promise<RootInfo[]> {
  return listOf(await fetch(rootsUrl(sessionId)), "roots");
}

export async function addRoot(sessionId: string, path: string, writable: boolean): Promise<RootsChange> {
  const res = await fetch(rootsUrl(sessionId), jsonBody("POST", { path, writable }));
  return res.json();
}

export async function removeRoot(sessionId: string, path: string): Promise<RootsChange> {
  const query = new URLSearchParams({ path }).toString();
  const res = await fetch(`${rootsUrl(sessionId)}?${query}`, { method: "DELETE" });
  return res.json();
}
