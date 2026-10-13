// Personas management: enable a persona, choose whether it shows in the new-session picker,
// set the default, and install more from a local directory or a GitHub repo (snapshotted).
// Re-skinned to the mock's Tailwind card idiom (§ Settings-as-page); the page title supplies the
// heading, so this drops its own "Personas" sub-header.
export const CHECK = "flex items-center gap-1.5 text-sm text-muted select-none shrink-0";
export const SELECT = "px-2.5 py-2 rounded-lg border border-line bg-paper text-md text-ink shrink-0";
export const INPUT =
  "flex-1 min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const BTN_BORDERED =
  "ctl border border-line bg-paper hover:border-lineStrong shrink-0 disabled:opacity-40 disabled:hover:border-line";
