import { Icon, ICON } from "../../Icon";
import type { SidebarModel } from "../model/useSidebarModel";

  // RECENT header with the group/filter control (§20) — the group toggle moved off the brand bar.
  // "Group by" flips the persona accordion ↔ chronological list; "Filter by assistant" narrows to
  // the checked personas (none checked = all shown).
  export function recentHeader(m: SidebarModel) {
  const { personas, hasHome, setSearchModalOpen, setGroupMenuOpen, groupMenuOpen, setGroupBy,
    layout, filterPersonas, setFilterPersonas, toggleFilterPersona } = m;
    const filterPersonaList = (personas || []).filter(hasHome);
    return (
    <div className="list-row relative flex items-center justify-between px-2" data-testid="recent-header">
      {/* Same heading as Pinned and Scheduled above it (.nav-sec). This was once mono caps
          while those were sentence case — two heading styles in one list, which reads as two
          different kinds of thing rather than two groups. Now all three are mono caps. */}
      <span className="nav-sec">Recent</span>
      <span className="flex items-center gap-0.5 -my-1 -mr-1">
        <button
          className="nav-icon-btn"
          data-tip="Search chats (⌘K)"
          aria-label="Search chats"
          data-testid="nav-search"
          onClick={() => setSearchModalOpen(true)}
        >
          <Icon name="search" size={ICON.md} />
        </button>
        <button
          className="nav-icon-btn"
          data-tip="Group & filter conversations"
          aria-label="Group and filter conversations"
          onClick={() => setGroupMenuOpen((v) => !v)}
        >
          <Icon name="sliders" size={ICON.md} />
        </button>
      </span>
      {groupMenuOpen && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setGroupMenuOpen(false)} />
          <div
            className="absolute right-0 top-7 z-50 w-56 rounded-xl border border-line bg-panel shadow-xl p-1.5"
            role="menu"
            data-testid="group-filter-menu"
          >
            <div className="px-2 pt-1 pb-1 text-xs text-faint font-semibold">
              Group by
            </div>
            {([["grouped", "Persona"], ["flat", "Chronological"]] as ["flat" | "grouped", string][]).map(
              ([key, label]) => (
                <button
                  key={key}
                  className="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-md text-left hover:bg-paper"
                  onClick={() => setGroupBy(key)}
                >
                  <span className="flex-1">{label}</span>
                  {layout === key && <span className="text-accent text-sm">✓</span>}
                </button>
              ),
            )}
            {filterPersonaList.length > 1 && (
              <>
                <div className="my-1 border-t border-line" />
                <div className="px-2 pt-1 pb-1 flex items-center justify-between">
                  <span className="text-xs text-faint font-semibold">
                    Filter by assistant
                  </span>
                  {filterPersonas.size > 0 && (
                    <button className="text-xs text-accent" onClick={() => setFilterPersonas(new Set())}>
                      Clear
                    </button>
                  )}
                </div>
                <div className="max-h-52 overflow-y-auto">
                  {filterPersonaList.map((p) => {
                    const checked = filterPersonas.has(p.id);
                    return (
                      <button
                        key={p.id}
                        className="w-full flex items-center gap-2 px-2 py-1.5 rounded-lg text-md text-left hover:bg-paper"
                        onClick={() => toggleFilterPersona(p.id)}
                      >
                        <span
                          className={
                            "w-3.5 h-3.5 rounded border grid place-items-center shrink-0 text-white " +
                            (checked ? "bg-accent border-accent" : "border-line")
                          }
                        >
                          {checked && <span className="text-xs leading-none">✓</span>}
                        </span>
                        <span className="flex-1 truncate">{p.name}</span>
                      </button>
                    );
                  })}
                </div>
                <div className="px-2 pt-1 pb-0.5 text-xs text-faint leading-snug">
                  None checked shows all.
                </div>
              </>
            )}
          </div>
        </>
      )}
    </div>
    );
  
}
