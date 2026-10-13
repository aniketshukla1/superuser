import { useState } from "react";
import { Icon, ICON } from "../Icon";
import { BTN_ACCENT } from "./INPUT";

/** One add-action, three doors behind it (SKILLS-SPEC §5): the list is the page. */
export function AddSkillMenu({ onLink, onCreateSkill }: { onLink: () => void; onCreateSkill?: (description: string) => void }) {
  const [addOpen, setAddOpen] = useState(false);
  return (
    <div className="relative shrink-0">
      <button
        className={BTN_ACCENT}
        aria-haspopup="menu"
        aria-expanded={addOpen}
        onClick={() => setAddOpen((v) => !v)}
      >
        <span className="inline-flex items-center gap-1.5">
          <Icon name="plus" size={ICON.md} /> Add skill
        </span>
      </button>
      {addOpen ? (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setAddOpen(false)} />
          <div
            role="menu"
            className="absolute right-0 top-full mt-1.5 w-80 rounded-xl2 border border-line bg-panel shadow-xl z-20 p-1.5"
            onKeyDown={(e) => e.key === "Escape" && setAddOpen(false)}
          >
            <button
              role="menuitem"
              className="w-full text-left px-3 py-2 rounded-lg hover:bg-paper"
              onClick={() => {
                setAddOpen(false);
                onLink();
              }}
            >
              <div className="text-md font-medium">Install from a link</div>
              <div className="text-xs text-muted">
                A GitHub repo or a URL to a SKILL.md — you read it before it installs
              </div>
            </button>
            <button
              role="menuitem"
              className="w-full text-left px-3 py-2 rounded-lg hover:bg-paper disabled:opacity-40"
              disabled={!onCreateSkill}
              onClick={() => {
                setAddOpen(false);
                onCreateSkill?.("");
              }}
            >
              <div className="text-md font-medium">Create with Superuser</div>
              <div className="text-xs text-muted">
                Starts a conversation — the assistant builds it and asks before adding it to
                your skills
              </div>
            </button>
          </div>
        </>
      ) : null}
    </div>
  );
}
