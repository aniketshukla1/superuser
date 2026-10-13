import { RAIL_NAV } from "../../../nav";
import { AttnBadge } from "../Props";
import { Icon, ICON, type IconName } from "../../Icon";
import type { ReactNode } from "react";
import type { SidebarModel } from "../model/useSidebarModel";

  // One renderer for both tiers, so a row cannot drift depending on which side of
  // the More disclosure it happens to sit on.
  export function railRows(m: SidebarModel, entries: typeof RAIL_NAV) {
  const { navTarget, totalAttention } = m;
  return entries.map((entry) => {
      const { go, active } = navTarget(entry.id);
      // Inbox carries the same attention count the account menu shows — the one
      // queue, counted once, wherever it is surfaced.
      const badge = entry.id === "inbox" ? <AttnBadge n={totalAttention} /> : null;
      return (
        <button
          key={entry.id}
          className={
            // Ink, not muted. design/Main.dc.html draws these dim, but the reference the
            // owner is matching keeps every destination bright and dims only the More
            // disclosure — and the artifact's way, applied to the whole sidebar, turned the
            // list into one grey wash with nothing to land on. Where the two disagree the
            // reference wins: these are places you go, not labels for other things.
            // "Active" is then carried by the background alone, which is how the reference
            // does it too.
            "nav-row w-full flex items-center gap-2.5 px-2 rounded-row text-md text-left text-ink hover:bg-solid " +
            (active ? "bg-line" : "")
          }
          data-testid={entry.testid}
          title={entry.hint}
          aria-current={active ? "page" : undefined}
          onClick={go}
        >
          <Icon name={entry.icon} size={ICON.md} className="shrink-0" />
          <span className="flex-1">{entry.label}</span>
          {badge}
        </button>
      );
    });

}

  // A row in the account menu (§26): closes the menu, then runs the destination.
  export function appMenuItem(m: SidebarModel, icon: IconName, label: string, onClick: () => void, active?: boolean, trailing?: ReactNode) {
  const { setAppMenuOpen } = m;
  return (
    <button
      role="menuitem"
      className={
        "w-full flex items-center gap-2.5 px-3 py-1.5 text-md text-left " +
        (active ? "text-ink bg-paper" : "hover:bg-paper")
      }
      aria-current={active ? "page" : undefined}
      onClick={() => {
        setAppMenuOpen(false);
        onClick();
      }}
    >
      <Icon name={icon} size={ICON.md} className="shrink-0 text-muted" />
      <span className="flex-1">{label}</span>
      {/* aria-hidden: the badge/shortcut must not leak into the accessible name (the old
          Inbox row's name-includes-the-badge-count nuisance, not repeated). */}
      {trailing != null && <span aria-hidden>{trailing}</span>}
    </button>
  );

}
