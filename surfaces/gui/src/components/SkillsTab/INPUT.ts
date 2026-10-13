// Settings ▸ Skills (SKILLS-SPEC §5/§6) — the management home: the LIST is the page; every
// add-surface appears only when summoned from the single "Add skill" menu (two doors: install
// from a link / start a conversation). Everything a user creates here is GLOBAL — "skills are
// things your worker knows everywhere". Creation-by-AI is a CONVERSATION (the worker proposes
// via save_skill) — there is no in-Settings drafting and no description box: the composer is
// where you describe it. Persona-bundled skills arrive with personas (§10), managed on the
// persona page, not here.
//
// The write form, file import, editing and "Show folder" came with the fork, built against
// routes this app's server never had, so each did nothing. They are gone until they are built.
// The on/off switch works.

export const INPUT =
  "w-full min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
export const BTN_ACCENT =
  "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const BTN_BORDERED =
  "ctl border border-line bg-paper hover:border-lineStrong shrink-0";
export const BADGE =
  "text-xs px-2 py-0.5 rounded-full border border-line bg-paper text-muted shrink-0";
