import { useEffect, useState, type RefObject } from "react";
import { getPersonas, type Persona } from "../../api";
import { MENTIONS, type Mention } from "./Props";

/** "@" mention popup. Deliberately narrower than the "/" one: it opens only while
 * the "@" is the FIRST character of the message and only while something matches.
 *
 * Both conditions are about not offering what would not work. The backend honours
 * a mention that opens the message and nothing else — `hello @brain` is a sentence
 * that mentions the brain, and routing it would mean a message could not talk
 * about the feature without invoking it. And a popup that stays open over "no
 * matches" swallows the Enter of anyone whose message simply starts with an email
 * address.
 *
 * After the brain come the other assistants: `@expenses …` hands that one message to
 * Expense Manager, who answers it here (`core::coworker::mentioned`). Fetched the first
 * time a message opens with "@", and matched on the handle or the name. */
export function useMentions(
  text: string,
  setText: (t: string) => void,
  textareaRef: RefObject<HTMLTextAreaElement | null>,
  agent?: string,
) {
  const [mentionIndex, setMentionIndex] = useState(0);
  const [personas, setPersonas] = useState<Persona[] | null>(null);
  const atStart = text.startsWith("@");
  useEffect(() => {
    if (!atStart || personas) return;
    let live = true;
    getPersonas()
      .then((rows) => live && setPersonas(rows))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [atStart, personas]);
  // Switched on, and not the one this conversation is already with: the backend treats
  // `@` + its own assistant as plain talk, so offering it would offer nothing.
  const others: Mention[] = (personas ?? [])
    .filter((p) => p.enabled !== false && p.id !== agent)
    .map((p) => ({ name: p.id, label: p.name, description: p.tagline ? `${p.name} — ${p.tagline}` : p.name }));
  const mentionQuery =
    atStart && !/\s/.test(text.slice(1)) ? text.slice(1).toLowerCase() : null;
  const mentionMatches =
    mentionQuery === null
      ? []
      : [...MENTIONS, ...others].filter(
          (m) => m.name.startsWith(mentionQuery) || !!m.label?.toLowerCase().startsWith(mentionQuery),
        );
  const mentionOpen = mentionQuery !== null && mentionMatches.length > 0;
  useEffect(() => {
    if (!mentionOpen) setMentionIndex(0);
  }, [mentionOpen]);
  // Inserted inline, like a picked skill: the "@name" text IS the state, so editing
  // it away un-picks it and there is nothing invisible to get out of step with.
  const pickMention = (name: string) => {
    setText(`@${name} `);
    textareaRef.current?.focus();
  };

  return { mentionIndex, setMentionIndex, mentionMatches, mentionOpen, pickMention };
}
