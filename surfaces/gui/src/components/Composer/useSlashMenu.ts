import { useEffect, useState, type RefObject } from "react";
import { sessionSkills, type SessionSkillRow } from "../../api";
import type { Attachment } from "../../types";
import { CONTEXT_COMMANDS } from "./Props";

/** "/" force-run (SKILLS-SPEC §4.1 #3). The popup derives from the draft: it is open while
 * the text is a bare "/query" (no whitespace yet) and no skill is picked. Selecting a row
 * inserts "/name " INLINE in the box (Claude-Code style — the slash text IS the state);
 * the user keeps typing after it, and on send the prefix is stripped while the skill name
 * rides the user_message as its own field. Editing the prefix away un-picks the skill. */
export function useSlashMenu({
  text,
  setText,
  sessionId,
  workspace,
  agent,
  running,
  onSend,
  textareaRef,
}: {
  text: string;
  setText: (t: string) => void;
  sessionId?: string;
  workspace?: string;
  agent?: string;
  running?: boolean;
  onSend: (text: string, attachments: Attachment[], skill?: string) => void;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
}) {
  const [pendingSkill, setPendingSkill] = useState<SessionSkillRow | null>(null);
  const [slashSkills, setSlashSkills] = useState<SessionSkillRow[] | null>(null);
  const [slashIndex, setSlashIndex] = useState(0);
  const prefixIntact =
    pendingSkill !== null &&
    (text === `/${pendingSkill.name}` || text.startsWith(`/${pendingSkill.name} `));
  useEffect(() => {
    if (pendingSkill && !prefixIntact) setPendingSkill(null);
  }, [pendingSkill, prefixIntact]);
  const slashQuery =
    !prefixIntact && sessionId && text.startsWith("/") && !/\s/.test(text.slice(1))
      ? text.slice(1).toLowerCase()
      : null;
  const slashMatches = (slashSkills ?? []).filter((s) =>
    s.name.toLowerCase().includes(slashQuery ?? ""),
  );
  // Built-in session commands. They take no arguments and never run a turn — the
  // backend answers them directly — so picking one sends it there and then, unlike
  // a skill, which only prefills the box for the user to keep typing after.
  const commandMatches =
    slashQuery === null ? [] : CONTEXT_COMMANDS.filter((c) => c.name.startsWith(slashQuery));
  // One index space over both groups. Skills come first so the default highlight
  // (and a bare "/" + Enter) still lands on a skill, as it did before commands existed.
  const choose = (i: number) => {
    const skill = slashMatches[i];
    if (skill) {
      pickSkill(skill);
      return;
    }
    const cmd = commandMatches[i - slashMatches.length];
    if (cmd) runCommand(cmd.name);
  };
  const optionCount = commandMatches.length + slashMatches.length;
  useEffect(() => {
    // Fetch on each popup open (fresh menu); drop when closed.
    if (slashQuery === null) {
      setSlashSkills(null);
      setSlashIndex(0);
      return;
    }
    if (slashSkills === null && sessionId) {
      sessionSkills(sessionId, workspace, agent)
        .then((all) => setSlashSkills(all.filter((s) => s.enabled)))
        .catch(() => setSlashSkills([]));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [slashQuery === null]);
  const pickSkill = (s: SessionSkillRow) => {
    setPendingSkill(s);
    setText(`/${s.name} `);
    textareaRef.current?.focus();
  };
  // A command IS the whole message: send it verbatim so the backend can answer it,
  // and never as a skill.
  const runCommand = (name: string) => {
    if (running) return;
    onSend(`/${name}`, [], undefined);
    setText("");
    setPendingSkill(null);
  };

  return {
    pendingSkill, setPendingSkill, prefixIntact, slashQuery, slashSkills, slashMatches, commandMatches,
    slashIndex, setSlashIndex, optionCount, choose, pickSkill, runCommand,
  };
}
