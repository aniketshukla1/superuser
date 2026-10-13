// Loading a saved conversation for display.
//
// Four places open a conversation — the sidebar, a link, switching persona, and resuming
// at launch — and each fetched the messages and mapped them itself. That was fine while a
// transcript was only its messages. It is now the messages plus the last turn's steps from
// the run journal, and a fifth copy of that pairing is exactly how one of the four would
// quietly stop showing steps.

import { getRunJournal, getSessionMessages, type ConversationMessage } from "./api";
import { toolCallsFrom } from "./components/ActivityTab";
import { itemsFromMessages, mergeJournalSteps } from "./itemsFromMessages";
import type { Item } from "./types";

export async function loadTranscript(sessionId: string): Promise<{ messages: ConversationMessage[]; items: Item[] }> {
  const [messages, journal] = await Promise.all([
    getSessionMessages(sessionId),
    // The journal is garnish: a conversation must open whether or not it can be read.
    getRunJournal(sessionId, 0, 500).catch(() => null),
  ]);
  const items = itemsFromMessages(messages);
  // A running turn replays its own steps over the socket; merging them here as well
  // would draw each one twice.
  if (!journal || journal.running) return { messages, items };
  return { messages, items: mergeJournalSteps(items, toolCallsFrom(journal.events)) };
}
