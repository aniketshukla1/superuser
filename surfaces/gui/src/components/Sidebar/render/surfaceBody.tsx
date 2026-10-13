import { Icon, ICON } from "../../Icon";
import { baseName } from "../../../paths";
import type { SidebarModel } from "../model/useSidebarModel";
import { sessionRow } from "./rows";
import { threadRows } from "./rows";

  // The expanded body for the active surface: a "New session" action, then the project-grouped
  // (or flat) session list, then the archived disclosure.
  export function surfaceBody(m: SidebarModel) {
  const { resolveOpenProject, normalizedQuery, projectOrder, filteredByProject, workspaceSurface,
    browseKey, projShowAll, peek, toggleProject, setProjShowAll, toggleSet, mine, matches,
    personaShowAll, setPersonaShowAll, archived, setShowArchived, showArchived, props } = m;
    const nestedOpenProject = resolveOpenProject(
      normalizedQuery
        ? projectOrder.filter((k) => (filteredByProject.get(k) || []).length > 0)
        : projectOrder,
    );
    return (
      <div className="space-y-1 px-1.5 pb-2 pt-0.5">
        {/* Body is flush inside the expanded group's fill (provided by the wrapper) so the header +
            its sessions read as one connected block — clear where a group ends and the next begins. */}
        {/* No per-persona "New session" here — the top split button's ▾ already starts a session
            in any persona (it was redundant + the mock's grouped cards don't have it). */}
        {workspaceSurface ? (
          <>
            {/* Codex-style Projects: a "+" header affordance, then collapsible folders whose
                rows carry a right-aligned compact age and truncate to PROJECT_PEEK + "Show more". */}
            <div className="flex items-center justify-between px-1.5 pt-1">
              <span className="font-mono text-xs text-faint font-medium">
                Projects
              </span>
              <button
                className="w-5 h-5 grid place-items-center rounded text-faint hover:text-ink hover:bg-panel"
                title="New project"
                aria-label="New project"
                onClick={() => props.onNewProject(browseKey)}
              >
                <Icon name="folderPlus" size={ICON.md} />
              </button>
            </div>
            <div className="space-y-0.5">
              {projectOrder.length === 0 && (
                <div className="px-2 py-1.5 text-sm text-faint leading-snug">
                  No projects yet — start one with the + above.
                </div>
              )}
              {projectOrder.map((proj) => {
                const list = filteredByProject.get(proj) || [];
                if (normalizedQuery && list.length === 0) return null; // hide non-matching folders while searching
                const isActive = proj === props.workspace;
                // This comment already called itself an accordion while behaving like a set
                // of independent toggles. Now it is one, and it is the same one the header
                // rows in Assistant use.
                const open = proj === nestedOpenProject;
                const showAll = !!normalizedQuery || projShowAll.has(proj);
                const shown = showAll ? list : list.slice(0, peek);
                return (
                  <div key={proj}>
                    <div
                      className={
                        "flex items-center gap-1.5 px-1.5 py-1 rounded-lg cursor-pointer select-none hover:bg-panel " +
                        (isActive ? "text-ink" : "text-muted hover:text-ink")
                      }
                      onClick={() => toggleProject(proj, nestedOpenProject)}
                      title={proj}
                    >
                      <Icon name="folder" size={ICON.md} className="shrink-0" />
                      {/* Same rule as the session rows: state is colour, not weight.
                          This switched 500 -> 600 for the folder you are in, on top
                          of the `text-ink` the parent row already switches to — two
                          signals for one fact, and the heavier one made a folder
                          name outweigh the session names beneath it. The folder
                          icon and the indent carry the hierarchy. */}
                      <span className="truncate min-w-0 text-md">{baseName(proj)}</span>
                      {/* Disclosure chevron sits AFTER the name (Codex parity), not leading the row. */}
                      <Icon
                        name={open ? "chevronDown" : "chevronRight"}
                        size={ICON.sm}
                        className="text-faint shrink-0"
                      />
                    </div>
                    {open &&
                      (list.length > 0 ? (
                        // pl-[19px] aligns each session's name under the folder NAME (folder icon
                        // 15 + gap 6 + row px 6 − session px 8 = 19), per Rohit's clean-column ask.
                        <div className="space-y-0.5 pl-[19px]">
                          {shown.map((s) => (
                            <div key={s.session_id}>
                              {sessionRow(m, s, { showTime: true })}
                              {threadRows(m, s)}
                            </div>
                          ))}
                          {!showAll && list.length > peek && (
                            <button
                              className="px-2 py-1 text-sm text-faint hover:text-muted"
                              onClick={() => setProjShowAll((s) => toggleSet(s, proj))}
                            >
                              Show more ({list.length - peek})
                            </button>
                          )}
                        </div>
                      ) : (
                        <div className="px-2 py-1.5 pl-[19px] text-sm text-faint leading-snug">
                          No conversations in this project yet.
                        </div>
                      ))}
                  </div>
                );
              })}
            </div>
          </>
        ) : (
          <div className="space-y-0.5">
            {mine.filter(matches).length === 0 ? (
              <div className="px-2 py-1.5 text-sm text-faint leading-snug">
                {normalizedQuery ? (
                  "No matching conversations."
                ) : (
                  <>
                    No conversations yet.{" "}
                    {/* An empty state that only reports the emptiness leaves you to find the
                        way out of it; the way out is one click and belongs here. */}
                    <button
                      className="text-accent hover:underline"
                      data-testid="empty-start-one"
                      onClick={() => props.onNewSession(browseKey)}
                    >
                      Start one
                    </button>
                  </>
                )}
              </div>
            ) : (
              <>
                {(personaShowAll.has(browseKey)
                  ? mine.filter(matches)
                  : mine.filter(matches).slice(0, peek)
                ).map((s) => (
                  <div key={s.session_id}>
                    {sessionRow(m, s)}
                    {threadRows(m, s)}
                  </div>
                ))}
                {!personaShowAll.has(browseKey) && mine.filter(matches).length > peek && (
                  <button
                    className="px-2 py-1 text-sm text-faint hover:text-muted"
                    onClick={() => setPersonaShowAll((s) => toggleSet(s, browseKey))}
                  >
                    Show more ({mine.filter(matches).length - peek})
                  </button>
                )}
              </>
            )}
          </div>
        )}

        {archived.length > 0 && (
          <div className="mt-2 pt-1.5 border-t border-line">
            <button
              className="w-full flex items-center gap-1.5 px-1.5 py-1 rounded text-sm text-faint hover:text-muted"
              onClick={() => setShowArchived((v) => !v)}
            >
              <Icon name={showArchived ? "chevronDown" : "chevronRight"} size={ICON.md} className="shrink-0" />
              Archived ({archived.length})
            </button>
            {showArchived && (
              <div className="space-y-0.5 mt-0.5">{archived.filter(matches).map((s) => sessionRow(m, s))}</div>
            )}
          </div>
        )}
      </div>
    );
  
}
