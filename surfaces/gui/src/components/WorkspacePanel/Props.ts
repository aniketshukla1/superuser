import type { TodoItem } from "../../types";
import type { IconName } from "../Icon";

export type FixedTab = "activity" | "assistant" | "files" | "board" | "terminal" | "browser" | "access";
export type ToolTab = "terminal" | "browser";
/** A document tab is addressed by its workspace-relative path. */
export type TabId = FixedTab | `doc:${string}`;

// Terminal and Browser are NOT here any more. They are tools you reach for, not places the
// panel is — a permanent tab for each meant two of the six were usually empty, and the strip
// they crowded is the one you scan to find what the turn produced. The topbar opens them
// (App.tsx), and opening one still lands here as a tab; it just is not standing there when
// nobody asked for it.
export const FIXED: { id: FixedTab; label: string; icon: IconName }[] = [
  { id: "activity", label: "Activity", icon: "clock" },
  { id: "assistant", label: "Assistant", icon: "sparkle" },
  { id: "files", label: "Files", icon: "folder" },
  // A shield, not sliders: this tab is what the assistant may touch, not a panel of knobs.
  { id: "access", label: "Access", icon: "shield" },
];

// A table, not a tick: the board is a list of work with columns, and a checkmark said
// "done" on a tab that mostly holds things that are not.
export const BOARD_TAB: { id: FixedTab; label: string; icon: IconName } = { id: "board", label: "Board", icon: "table" };

export const TOOL_TABS: { id: ToolTab; label: string; icon: IconName }[] = [
  { id: "terminal", label: "Terminal", icon: "code" },
  { id: "browser", label: "Browser", icon: "globe" },
];

/** Which tabs the strip shows.
 *
 * Terminal and Browser are absent until the topbar opens one — they are tools you reach
 * for, not places the panel is, and a permanent tab for each meant two of six stood empty
 * in the strip you scan to find what a turn produced. Once opened, a tool IS a tab, so you
 * can switch away and back without losing the shell you were in.
 *
 * A LIST, not one tool. This used to hold a single id, so opening the browser took the
 * terminal's tab away — while its pane stayed mounted and its shell stayed alive, because
 * both panes have always been rendered and merely hidden. Nothing was ever being closed;
 * the strip was just refusing to admit that two things were open.
 *
 * Ordered by TOOL_TABS, not by when you opened them, so the strip does not reshuffle
 * under your cursor the second time you reach for one.
 *
 * Split out of the component because it is the whole rule and the component needs a
 * terminal and a webview to render.
 */
export function panelTabs(openTools: readonly ToolTab[] | ToolTab | null, hasBoard = false) {
  // Tolerates the old single-value shape so a caller that has not been updated still
  // gets a sensible strip rather than an empty one.
  const open: readonly ToolTab[] =
    openTools == null ? [] : typeof openTools === "string" ? [openTools] : openTools;
  // Board shows for any session with a project, empty or not.
  //
  // It was hidden until the project had one, by analogy with Terminal and Browser —
  // which was the wrong analogy. Those are hidden AND have topbar buttons that
  // reveal them, so you can find them on purpose. The board had no such affordance,
  // so "hidden until it exists" meant "undiscoverable until something else creates
  // it", and nothing tells you the feature is there. An empty tab that explains what
  // a board is teaches the feature; an absent one hides it.
  return [...FIXED, ...(hasBoard ? [BOARD_TAB] : []), ...TOOL_TABS.filter((t) => open.includes(t.id))];
}

/** What a press of a topbar tool button does.
 *
 * Three outcomes, and which one you get is exactly what the button looks like it will do:
 *
 *   not open        → open it, go there
 *   open, elsewhere → go there. Pressing "Terminal" from the Files tab means "show me the
 *                     terminal", never "throw it away"
 *   open, showing   → put it away, and land back where you were before you reached for it
 *
 * The button is lit in precisely the third case, so a lit button closes and an unlit one
 * opens — which is what a toggle has to mean to be worth having.
 *
 * "Put away" is the TAB, not the process. The shell keeps running and the page keeps its
 * state; reopening puts you back where you were. A misfired double-press should not kill
 * a running command.
 *
 * Split out of the component for the same reason `panelTabs` is: it is the whole rule,
 * and the component needs a terminal and a webview to render.
 */
export function toolPress(
  tool: ToolTab,
  showing: string,
  open: readonly ToolTab[],
  fallback: string,
): { open: ToolTab[]; tab: string } {
  if (showing === tool) return { open: open.filter((t) => t !== tool), tab: fallback };
  return { open: open.includes(tool) ? [...open] : [...open, tool], tab: tool };
}

export interface Props {
  /** A tool the topbar asked to open, with a nonce so pressing the same button twice
   *  re-opens it. Without the nonce, clicking Terminal after switching away by hand would
   *  do nothing — the prop would not have changed. */
  openRequest?: { tool: ToolTab; n: number } | null;
  active: boolean;
  sessionId: string;
  refreshKey: number;
  todo: TodoItem[];
  running: boolean;
  personaId?: string;
  workspace?: string;
  branch?: string | null;
  scratchPrimary?: boolean;
  /** Bumping this selects the Access tab (the §32 deep link). */
  openAccessKey?: number;
  /** Bumping this selects the Files tab — the topbar's Artifacts button. Without it
   *  that button only un-hid the rail, landing you on whichever tab was last open
   *  (Activity, usually) with no files in sight, and the button itself gone because
   *  it only shows while the rail is hidden. You pressed "Artifacts", got Activity,
   *  and lost the way back. */
  openFilesKey?: number;
  /** Sends a message as the user. The Assistant tab's "+ Add" buttons compose a
   *  prompt rather than opening a form, so the panel needs the composer. */
  onSend?: (text: string) => void;
  onOpenIntegrations?: () => void;
  /** Which tab the panel is showing. The topbar's Terminal and Browser buttons open
   *  a tool and then looked identical whether or not it was open — the only way to
   *  tell was to look at the panel, which is what the button was for. It cannot know
   *  on its own: the panel owns this state. */
  onTabChange?: (tab: string) => void;
  /** Which tools are open and holding something — a shell still running, a page
   *  still loaded — whether or not the panel is showing them. The topbar marks
   *  them, the way Artifacts marks files you have not looked at. The panel owns
   *  this: both panes stay mounted once opened, so "open" is a fact only it has. */
  onToolsChange?: (tools: string[]) => void;
}
