// The document renderers — markdown, html, image, pdf, csv, xlsx, folder listings.
//
// These used to live inside RightRail.tsx, where opening a file took the whole rail
// over ("artifact-mode") and the app had to auto-collapse the left nav to make room.
// In the tabbed WorkspacePanel each open file is its own tab, so the viewer renders
// as tab content and the back button is gone — the tab strip is the way back.

export { kindIcon, kindFromPath, kindLabel, formatBytes, formatTime } from "./ArtifactViewer/kindLabel";
export { ArtifactViewer } from "./ArtifactViewer/ArtifactViewer";
