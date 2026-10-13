import type { KeyboardEvent } from "react";
import type { SessionSkillRow } from "../../api";
import type { Attachment } from "../../types";
import { CONTEXT_COMMANDS, type MENTIONS } from "./Props";

type Ctx = {
  text: string;
  setText: (t: string) => void;
  attachments: Attachment[];
  setAttachments: (a: Attachment[]) => void;
  pendingSkill: SessionSkillRow | null;
  setPendingSkill: (s: SessionSkillRow | null) => void;
  prefixIntact: boolean;
  slashQuery: string | null;
  slashIndex: number;
  setSlashIndex: (update: (i: number) => number) => void;
  optionCount: number;
  choose: (i: number) => void;
  runCommand: (name: string) => void;
  mentionOpen: boolean;
  mentionMatches: typeof MENTIONS;
  mentionIndex: number;
  setMentionIndex: (update: (i: number) => number) => void;
  pickMention: (name: string) => void;
  recording: boolean;
  dictationBusy: string | null;
  needsModel: boolean;
  onConnectModel?: () => void;
  onSend: (text: string, attachments: Attachment[], skill?: string) => void;
};

/** Sending, and the keys the box answers: the "/" and "@" menus first, then Enter. */
export function composerKeys(c: Ctx) {
  const {
    text, setText, attachments, setAttachments, pendingSkill, setPendingSkill, prefixIntact,
    slashQuery, slashIndex, setSlashIndex, optionCount, choose, runCommand,
    mentionOpen, mentionMatches, mentionIndex, setMentionIndex, pickMention,
    recording, dictationBusy, needsModel, onConnectModel, onSend,
  } = c;

  const submit = () => {
    // A fully typed command sends as itself — the popup is showing it, but the draft
    // is complete, so Enter must not be swallowed the way a half-typed skill name is.
    const typedCommand = CONTEXT_COMMANDS.find((c) => text.trim() === `/${c.name}`);
    if (typedCommand) {
      runCommand(typedCommand.name);
      return;
    }
    // While the "/" popup is open the draft is a query, not a message — never send it.
    if (slashQuery !== null) return;
    // Same for "@": "@bra" is a half-typed target, not a question for it.
    if (mentionOpen) return;
    // The visible "/name " prefix is UI state, not message text — strip it for the send;
    // the skill rides as its own field.
    const skill = prefixIntact ? pendingSkill!.name : undefined;
    const t = (skill ? text.slice(skill.length + 1) : text).trim();
    // `running` is deliberately NOT a reason to refuse. A message typed mid-turn
    // steers the agent instead of being dropped — the moment you can see it going
    // wrong is exactly the moment you want to say so.
    if ((!t && attachments.length === 0 && !skill) || recording || dictationBusy) return;
    // No model connected: keep the draft (don't drop it) and send the user to setup instead.
    if (needsModel) {
      onConnectModel?.();
      return;
    }
    onSend(t, attachments, skill);
    setText("");
    setAttachments([]);
    setPendingSkill(null);
  };

  const onKey = (e: KeyboardEvent) => {
    if (slashQuery !== null) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashIndex((i) => Math.min(i + 1, Math.max(optionCount - 1, 0)));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashIndex((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setText("");
        return;
      }
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        // A command typed out in full sends itself even if the popup still lists it.
        if (CONTEXT_COMMANDS.some((c) => text.trim() === `/${c.name}`)) {
          submit();
          return;
        }
        choose(slashIndex);
        return;
      }
    }
    if (mentionOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionIndex((i) => Math.min(i + 1, mentionMatches.length - 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionIndex((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setText("");
        return;
      }
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        pickMention((mentionMatches[mentionIndex] ?? mentionMatches[0]).name);
        return;
      }
      // Tab completes too. A menu you drive with the arrow keys is a menu people
      // also try to Tab through, and the alternative here is focus leaving the box.
      if (e.key === "Tab") {
        e.preventDefault();
        pickMention((mentionMatches[mentionIndex] ?? mentionMatches[0]).name);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  };

  return { submit, onKey };
}
