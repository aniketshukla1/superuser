import { useState } from "react";
import type { Persona } from "../../api";
import { shortPersonaName } from "../../personaScope";
import { Icon, ICON } from "../Icon";
import { PersonaGlyph } from "../personaIcon";
import { showPersonas } from "../../flags";

// New-session split (§8): the primary action starts a session with the persona you are
// looking at (`current`); the ▾ opens a menu of the enabled personas (from /v1/personas)
// plus "New assistant…" and "Manage personas…".
//
// It used to be a full-width filled iris button sitting above the nav. That gave the most
// routine action in the app the loudest treatment on the screen — louder than the session
// you are actually reading — and it broke the sidebar's rhythm, because every other place
// you can go is a 13px row. design/Main.dc.html has it as the FIRST such row, in ink while
// its neighbours are muted: primary by contrast rather than by area.
//
// The ▾ survives that change deliberately. It is not decoration: it is how you start a
// session as a specific persona, and it is the only way to do so. A "plain New row like the
// reference" that dropped it would be matching a screenshot of an app that has one kind of
// session. So this is still a split control — same two jobs, at row weight instead of
// button weight, with the chevron always rendered (not hover-revealed) so it stays
// reachable by keyboard.
//
// The visible label is "New"; the accessible name stays "New session". WCAG 2.5.3 wants the
// accessible name to contain the visible label, which it does, and "New" alone would be a
// poor thing to hear announced.
export function NewSessionSplit({
  personas,
  current,
  onNew,
  onManage,
  onNewAssistant,
  onNewProject,
}: {
  personas: Persona[] | null;
  current: string;
  onNew: (agent: string) => void;
  onManage: () => void;
  onNewAssistant: () => void;
  onNewProject: () => void;
}) {
  const [open, setOpen] = useState(false);
  // `surfaced` IS this menu. Settings calls the control "In picker" and says it
  // decides "whether it appears in the new-session picker" — and this menu is the
  // new-session picker. It filtered on `enabled` alone, so turning an assistant off
  // here changed the sidebar and left this list untouched: you could still pick the
  // assistant you had just hidden, and doing so opened a conversation the sidebar
  // then had no section for.
  const enabled = (personas || []).filter((p) => p.enabled && p.surfaced);
  // With a single enabled persona there is nothing to pick — the split collapses to a plain
  // button (owner ask 2026-07-09). `personas === null` (still loading) keeps the split so the
  // control doesn't visibly change shape once the list arrives with 2+.
  const solo = personas !== null && enabled.length <= 1;
  return (
    <div className="relative">
      <div className="flex items-center gap-0.5">
        <button
          className="newsplit-primary nav-row flex-1 min-w-0 flex items-center gap-2.5 px-2 rounded-row text-md text-left text-ink hover:bg-solid"
          title="New session (⌘N)"
          aria-label="New session"
          onClick={() => onNew(solo && enabled.length === 1 ? enabled[0].id : current)}
        >
          <Icon name="plusCircle" size={ICON.md} className="shrink-0" />
          <span className="flex-1">New</span>
        </button>
        {!solo && (
          <button
            className="w-6 h-6 grid place-items-center rounded-row text-faint hover:text-ink hover:bg-solid shrink-0"
            title="Start with a specific persona"
            aria-label="Choose a persona"
            aria-haspopup="menu"
            aria-expanded={open}
            onClick={() => setOpen((v) => !v)}
          >
            <Icon name="chevronDown" size={ICON.md} />
          </button>
        )}
      </div>
      {open && (
        <>
          <div className="fixed inset-0 z-20" onClick={() => setOpen(false)} />
          {/* Wider than the rail. Bound to the rail's own width, every tagline was cut after
              about three words — "Say where the money actuall…" — in a menu whose only job is
              to tell the assistants apart. */}
          <div className="newsplit-menu absolute left-0 mt-1 z-30 bg-panel border border-line rounded-xl2 shadow-xl p-1">
            <div className="px-2 py-1 text-xs text-faint font-semibold">
              Start a session as
            </div>
            {enabled.map((p) => (
              <button
                key={p.id}
                className="w-full flex items-center gap-2.5 px-2 py-1.5 rounded-lg hover:bg-paper text-left"
                onClick={() => {
                  setOpen(false);
                  onNew(p.id);
                }}
              >
                <span className="w-6 h-6 rounded-md bg-paper border border-line grid place-items-center text-muted shrink-0">
                  <PersonaGlyph icon={p.icon} family={p.family} size={ICON.sm} />
                </span>
                <span className="min-w-0">
                  <span className="block text-md font-medium truncate">
                    {shortPersonaName(p.name, p.id)}
                  </span>
                  {p.tagline && <span className="newsplit-tagline">{p.tagline}</span>}
                </span>
              </button>
            ))}
            <div className="border-t border-line mt-1 pt-1">
              {/* New project lived on a folder-plus button beside a "PROJECTS" heading. Both
                  are gone — the artifact has no such heading — so the action comes here, next
                  to New assistant, which is the same kind of decision: make a new thing to
                  work in. Per-project + is unaffected; that one starts a session in a folder
                  that already exists. */}
              <button
                className="w-full px-2 py-1.5 rounded-lg hover:bg-solid text-left text-md text-accent"
                data-testid="menu-new-project"
                onClick={() => {
                  setOpen(false);
                  onNewProject();
                }}
              >
                + New project…
              </button>
              <button
                className="w-full px-2 py-1.5 rounded-lg hover:bg-solid text-left text-md text-accent"
                onClick={() => {
                  setOpen(false);
                  onNewAssistant();
                }}
              >
                + New assistant…
              </button>
              {showPersonas() && (
                <button
                  className="w-full px-2 py-1.5 rounded-lg hover:bg-paper text-left text-md text-muted"
                  onClick={() => {
                    setOpen(false);
                    onManage();
                  }}
                >
                  Manage personas…
                </button>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
