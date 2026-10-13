import { baseName } from "../../../paths";
import { Icon, ICON } from "../../Icon";
import type { SidebarModel } from "../model/useSidebarModel";
import { sessionRow } from "./rows";

  export function projectFirstList(m: SidebarModel) {
  const { normalizedQuery, codeProjectOrder, codeByProject, resolveOpenProject, projShowAll, peek,
    toggleProject, projectAgent, setProjShowAll, toggleSet, props } = m;
    // A search drops folders with no hits. The nested list has always done this; the
    // top-level one instead force-opened EVERY folder, which an accordion cannot do —
    // and which left you scanning folders that could not contain your result anyway.
    const codeShown = normalizedQuery
      ? codeProjectOrder.filter((proj) => (codeByProject.get(proj) || []).length > 0)
      : codeProjectOrder;
    const codeOpenProject = resolveOpenProject(codeShown);
    return (
    <div className="space-y-0.5" data-testid="project-first">
      {codeProjectOrder.length === 0 && (
        <div className="px-2 py-1.5 text-sm text-faint leading-snug">
          {normalizedQuery ? "No matching sessions." : "No projects yet — start one with + above."}
        </div>
      )}
      {codeShown.map((proj, gi) => {
        const list = codeByProject.get(proj) || [];
        const open = proj === codeOpenProject;
        const showAll = !!normalizedQuery || projShowAll.has(proj);
        const shown = showAll ? list : list.slice(0, peek);
        const isActive = !!proj && proj === props.workspace;
        return (
          <div key={proj || "__none__"} className={gi > 0 ? "mt-3" : undefined}>
          {/* Expanded, the wrapper carries the recessed fill so the folder name sits INSIDE
              the block with its sessions — one connected group, with a clear edge where it
              ends and the next folder begins. This is the persona groups' wrapper, not a
              lookalike: the two lists are the same gesture on the same kind of thing, and
              the whole point of this change is that they should stop looking like two
              different widgets. */}
          <div className={open ? "rounded-xl bg-paper/70 overflow-hidden" : ""}>
            {/* A project header is a LABEL, not a control: the folder name in muted 12.5px
                with a + at the end, exactly as design/Main.dc.html and the reference both
                have it. It carried a folder icon, a rotating chevron and a session count —
                four pieces of chrome on a row whose job is to say one word — and the count
                in particular restated something the rows underneath already showed.

                I removed the collapse chevron with them, reasoning that `peek` already caps
                each group. That was wrong for anyone with several projects open: capping a
                list is not the same as being able to put one away, and Home's persona groups
                collapse — so Workspace's folders behaving differently was an inconsistency,
                not a simplification. The chevron is back, at the right where Home puts it. */}
            <div
              className={
                "nav-row flex items-center gap-2.5 px-2 cursor-pointer select-none " +
                (open ? "" : isActive ? "rounded-lg bg-paper" : "rounded-lg hover:bg-paper")
              }
              title={proj || "Sessions not yet bound to a folder"}
              role="button"
              tabIndex={0}
              aria-expanded={open}
              onClick={() => toggleProject(proj, codeOpenProject)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  toggleProject(proj, codeOpenProject);
                }
              }}
            >
              {/* No leading folder icon, though the persona headers carry a glyph. That is
                  the one place these two rows are deliberately allowed to differ: a persona
                  glyph tells assistants apart, whereas a folder icon on a list of folders
                  says only "folder" — it was removed for that reason and Sidebar.test.tsx
                  holds the line at two svgs. Aligning the rows is worth doing; putting back
                  chrome that was cut on purpose is not. */}
              {/* Muted even for the folder you are in: brightening it made the group label
                  compete with the session titles under it, and which folder you are in is
                  already obvious from the session that is lit inside it. */}
              <span className="truncate min-w-0 flex-1 text-sm text-muted">
                {proj ? baseName(proj) : "No folder"}
              </span>
              {/* Start a session already in this folder. onNewProject opens the folder
                  gate because no folder is known; here one is, so the gate would just
                  ask a question we can already answer.

                  Which assistant? Whichever workbench you are already in, so the button
                  does not silently switch you; otherwise the one you last used in THIS
                  folder, which is the only other answer the folder can give. Rendered
                  always rather than on hover so it is reachable by keyboard, and never
                  on "No folder" — there is no folder to open a session in. */}
              {proj && (
                <button
                  className="nav-icon-btn nav-row-plus"
                  data-testid="project-new"
                  aria-label={`New session in ${baseName(proj)}`}
                  title={`New session in ${baseName(proj)}`}
                  onClick={(e) => {
                    e.stopPropagation(); // the row behind it toggles the folder
                    props.onNewInProject(proj, projectAgent(list));
                  }}
                >
                  <Icon name="plus" size={ICON.md} />
                </button>
              )}
              <Icon
                name={open ? "chevronDown" : "chevronRight"}
                size={ICON.md}
                className="text-faint shrink-0"
              />
            </div>
            {open && (
              <div className="space-y-0.5 px-1.5 pb-2 pt-0.5">
                {shown.map((s) => sessionRow(m, s, { showTime: false }))}
                {list.length > shown.length && (
                  <button
                    className="list-row w-full text-left px-2 text-sm text-faint hover:text-ink"
                    onClick={() => setProjShowAll((v) => toggleSet(v, proj))}
                  >
                    Show {list.length - shown.length} more
                  </button>
                )}
              </div>
            )}
          </div>
          </div>
        );
      })}
    </div>
    );
  
}
