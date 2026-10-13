import type { RefObject } from "react";
import { chooseFolder } from "../../tauri";

/** What the "+" menu does: open the file picker, put a folder's path in the message, or
 * start a "/" command. */
export function useComposerActions(
  fileInput: RefObject<HTMLInputElement | null>,
  textareaRef: RefObject<HTMLTextAreaElement | null>,
  setText: (update: (draft: string) => string) => void,
) {
  // The "+" menu offers typed shortcuts; each just narrows the OS picker's filter.
  const pickFiles = (accept: string) => {
    if (fileInput.current) {
      fileInput.current.accept = accept;
      fileInput.current.click();
    }
  };

  /** One "Add files or photos" row instead of three by type. The three were a menu
   *  asking you to classify your file before you had picked it — and the picker asks
   *  again anyway. Everything the composer can read, in one dialog. */
  const ATTACH_ACCEPT =
    "image/*,application/pdf,.pdf,text/*,.md,.csv,.json,.yaml,.yml,.log,.py,.ts,.tsx,.js,.rs,.go,.toml";

  /** A folder cannot be attached — there is no upload for a directory — so this puts its
   *  path in the message, which is the form the agent can actually act on. */
  const addFolder = async () => {
    const dir = await chooseFolder().catch(() => null);
    if (!dir) return;
    setText((draft) => (draft.trim() ? `${draft.trimEnd()} ${dir}` : dir));
    textareaRef.current?.focus();
  };

  /** The slash popup is already wired to the "/" the textarea sees; the menu row types
   *  it for you rather than duplicating the popup somewhere else. */
  const openSlashCommands = () => {
    setText((draft) => (draft.startsWith("/") ? draft : `/${draft}`));
    textareaRef.current?.focus();
  };

  return { pickFiles, ATTACH_ACCEPT, addFolder, openSlashCommands };
}
