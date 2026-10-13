// The workspace panel — the session's one side surface, as tabs.
//
// It replaces the stacked RightRail (Progress / Artifacts / Run journal / Access).
// Those sections didn't go away, they changed altitude:
//   Progress + Run journal → Activity, as tool calls rather than raw frames
//   Artifacts              → Files, with what this run changed pulled to the top
//   Access                 → a tab of its own, alongside the rest
// and two tabs are new: a real Terminal (the shell the agent shares) and a real
// Browser (a child webview, not an iframe).
//
// Opening a file no longer takes the panel over — each one is a closable tab.

export { panelTabs, toolPress } from "./WorkspacePanel/Props";
export { WorkspacePanel } from "./WorkspacePanel/WorkspacePanel";
