import { Icon, ICON } from "../../Icon";
import { scheduleLabel, scheduleShort } from "../../../scheduleText";
import { UnseenBadge } from "../Props";
import type { SidebarModel } from "../model/useSidebarModel";
import { cardRow } from "./cardRow";

  // The cross-persona Pinned band (manual pins only) — icon-free rows. Appears in BOTH layouts
  // (flat list AND accordion), so it's factored here for reuse.
  export function pinnedBand(m: SidebarModel) {
  const { pinnedSessions } = m;
  return pinnedSessions.length > 0 ? (
      <div>
        <div className="px-1.5 nav-sec mb-1">
          Pinned
        </div>
        <div className="space-y-0.5">
          {pinnedSessions.map((s) => cardRow(m, s))}
        </div>
      </div>
    ) : null;

}

  // UX-023: the Scheduled band — ONE entry per automation (never per run): name +
  // cadence, with the unseen-runs badge. Runs themselves never enter Recent (run
  // sessions are __run__-prefixed and hidden from the sessions list).
  export function scheduledBand(m: SidebarModel) {
  const { automations, setScheduledOpen, scheduledOpen, props } = m;
  return automations.length > 0 ? (
      // Scheduled is a peer of the project groups, not a separate band with its own
      // typographic style: same 12.5px muted header, same row shape, same pip. It used a
      // 10px mono ALL-CAPS heading and a two-line row, which made a handful of automations
      // look like a different feature bolted to the bottom of the list. design/Main.dc.html
      // has it as just another group, and the cadence folded onto the title line.
      <div data-testid="scheduled-band">
        {/* The same header shape an assistant group has: click the row to fold it,
            a "+" at rest to make a new one, a chevron saying which way it is. It
            had neither, so the one band you cannot add to from the sidebar was the
            one whose whole purpose is things that run without you. */}
        {/* The row stays clickable for a pointer, but the control is the button inside it. The
            row used to BE the control (role="button") with the "+" button nested in it — an
            interactive element inside another, which a screen reader announces as one merged
            "Scheduled New automation" button and keyboard users could not reliably reach.
            The button was only as tall as its 16px label, under the 24px target floor, so it
            borrows the row's padding (py-1 -my-1): a 24px target, the row's height unchanged. */}
        <div
          className="list-row flex items-center gap-1.5 px-2 rounded-row cursor-pointer hover:bg-solid"
          onClick={() => setScheduledOpen((v) => !v)}
        >
          <button
            type="button"
            className="flex-1 min-w-0 py-1 -my-1 text-left nav-sec"
            aria-expanded={scheduledOpen}
            data-testid="scheduled-header"
            onClick={(e) => {
              e.stopPropagation();
              setScheduledOpen((v) => !v);
            }}
          >
            Scheduled
          </button>
          <button
            className="nav-icon-btn nav-row-plus"
            title="New automation"
            aria-label="New automation"
            data-testid="scheduled-new"
            onClick={(e) => {
              e.stopPropagation();
              props.onNewAutomation();
            }}
          >
            <Icon name="plus" size={ICON.md} />
          </button>
          <Icon
            name={scheduledOpen ? "chevronDown" : "chevronRight"}
            size={ICON.md}
            className="text-faint shrink-0"
          />
        </div>
        <div className="sched-list space-y-0.5" data-testid="scheduled-list" hidden={!scheduledOpen}>
          {automations.map((a) => (
            <button
              key={a.id}
              className="list-row w-full flex items-center gap-2 px-2 rounded-row text-left hover:bg-solid"
              data-testid={`scheduled-${a.id}`}
              data-tip={`${a.title} — ${scheduleLabel(a.schedule)}`}
              onClick={() => props.onOpenAutomation(a.id)}
            >
              <span className="w-[5px] h-[5px] rounded-full border border-faint shrink-0" aria-hidden />
              {/* Two spans, not one string: the artifact's example cadence was "7:00" and fit
                  beside the title, but a real one is "Every day at ~7:00 AM" and a single
                  truncating line eats the NAME of the automation — the one part you need to
                  tell them apart. The title takes the space first; the cadence gets what is
                  left and truncates itself. Both are in the row's tooltip (data-tip). */}
              <span className="flex-1 min-w-0 row-name text-ink truncate">{a.title}</span>
              {/* The SHORT cadence ("Mon 9:30 am"), flex-none + a cap, NOT `shrink`. The full
                  sentence ("Weekdays at about 8:00 am") never fit beside a name in a 240px
                  sidebar, so both clipped — "Morning…  Weekdays at ab…". The short one does, and
                  the sentence is in the row's tooltip. Below 280px of row it steps aside entirely
                  (base-sidebar.css), so a name is never clipped to make room for its time. */}
              <span className="sched-when flex-none max-w-[46%] text-xs text-faint truncate">
                {scheduleShort(a.schedule)}
              </span>
              <UnseenBadge n={a.unseen_runs || 0} failed={a.unseen_failed} />
            </button>
          ))}
        </div>
      </div>
    ) : null;

}
