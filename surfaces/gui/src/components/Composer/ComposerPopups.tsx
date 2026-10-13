import type { SessionSkillRow } from "../../api";
import type { CONTEXT_COMMANDS, MENTIONS } from "./Props";
import { SlashRow } from "./SlashRow";

/** The "/" list — skills, then commands — floating above the input. */
export function SlashPopup({
  slashSkills,
  slashMatches,
  commandMatches,
  slashIndex,
  setSlashIndex,
  pickSkill,
  runCommand,
}: {
  slashSkills: SessionSkillRow[] | null;
  slashMatches: SessionSkillRow[];
  commandMatches: typeof CONTEXT_COMMANDS;
  slashIndex: number;
  setSlashIndex: (i: number) => void;
  pickSkill: (s: SessionSkillRow) => void;
  runCommand: (name: string) => void;
}) {
  return (
    <div
      className="slash-popup"
      data-testid="skill-popup"
      role="listbox"
      aria-label="Commands"
    >
      {slashSkills === null ? (
        <div className="slash-empty">Loading skills…</div>
      ) : slashMatches.length === 0 && commandMatches.length === 0 ? (
        <div className="slash-empty">No matching commands.</div>
      ) : null}
      {/* A name, then what it does — on ONE line, so the list is still scanned by
          looking rather than read. The rows carried the name alone, which works for the
          command you already know and leaves "api-design" meaning nothing to anyone
          meeting it; they also carried a scope pill and a second line, which is what
          made the list a wall in the first place. The full text stays in the tooltip. */}
      {slashMatches.map((sk, i) => (
        <SlashRow
          key={`s-${sk.name}`}
          name={sk.name}
          hint={`${sk.description} · ${sk.scope}`}
          active={i === slashIndex}
          onHover={() => setSlashIndex(i)}
          onPick={() => pickSkill(sk)}
        />
      ))}
      {commandMatches.map((c, i) => (
        <SlashRow
          key={`c-${c.name}`}
          name={c.name}
          hint={`${c.description} · command`}
          active={slashMatches.length + i === slashIndex}
          onHover={() => setSlashIndex(slashMatches.length + i)}
          onPick={() => runCommand(c.name)}
        />
      ))}
    </div>

  );
}

/** The "@" list. Same panel and same rows as "/" on purpose — they are two menus of the
 * same kind, and giving the second one a look of its own would say it behaves differently. */
export function MentionPopup({
  mentionMatches,
  mentionIndex,
  setMentionIndex,
  pickMention,
}: {
  mentionMatches: typeof MENTIONS;
  mentionIndex: number;
  setMentionIndex: (i: number) => void;
  pickMention: (name: string) => void;
}) {
  return (
    <div className="slash-popup" data-testid="mention-popup" role="listbox" aria-label="Mentions">
      {mentionMatches.map((m, i) => (
        <SlashRow
          key={`m-${m.name}`}
          name={`@${m.name}`}
          hint={m.description}
          active={i === mentionIndex}
          onHover={() => setMentionIndex(i)}
          onPick={() => pickMention(m.name)}
        />
      ))}
    </div>

  );
}
