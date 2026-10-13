/** Where cancelling the folder gate goes.
 *
 * Raising the gate is a NAVIGATION, not a question asked in place. `startNewSession`,
 * `newProject` and `switchAgent` each switch the agent and clear the workspace BEFORE
 * the dialog appears, because a workbench must not inherit the previous persona's
 * folder. So by the time you see "Choose a project folder", the app has already left
 * the conversation you were in.
 *
 * Cancel has to undo that. It used to invent a destination instead — the general
 * assistant — so declining the dialog put you on Home looking at a different chat.
 * Reported as: "when selected the folder or close the dialog the screen moves to the
 * Home automatically, this should have stayed in the workspace folder".
 *
 * The decision lives here rather than inline in App.tsx because one of its branches is
 * a trap, and a trap is worth naming: returning to a workbench that has no folder
 * restores the gate's own precondition, so the dialog you just dismissed comes back —
 * or, worse, does not, and leaves a screen that cannot do anything.
 */

/** The state the gate navigated away from, captured at the moment it was raised. */
export type GateReturn = {
  agent: string;
  workspace: string | null;
  sessionId: string;
  branch: string | null;
};

export type CancelTarget =
  /** Reopen the conversation the gate interrupted. */
  | { kind: "resume"; agent: string; workspace: string; sessionId: string; branch: string | null }
  /** Nothing to return to and nowhere to stand: switch to an assistant, which needs no folder. */
  | { kind: "fallback" }
  /** The gate was raised over a working screen (change-folder). Closing it IS the action. */
  | { kind: "stay" };

export function gateCancelTarget(
  back: GateReturn | null,
  /** The workspace that is live right now — null while a gate raised by navigation is up. */
  liveWorkspace: string | null,
  gatesWorkspace: (agent: string) => boolean,
): CancelTarget {
  // A return point is only usable if the screen it names can actually be shown. A
  // workbench with no folder is exactly the state the gate exists to prevent, so
  // "returning" to one would hand back the dead screen instead of undoing anything.
  const usable = !!back?.sessionId && (!!back.workspace || !gatesWorkspace(back.agent));
  if (back && usable) {
    return {
      kind: "resume",
      agent: back.agent,
      workspace: back.workspace ?? "",
      sessionId: back.sessionId,
      branch: back.branch,
    };
  }
  if (liveWorkspace) return { kind: "stay" };
  return { kind: "fallback" };
}

/** The assistant to land on when there is nothing to go back to — a true first run.
 *
 * Never a workbench. A workbench needs a folder, which is the question just declined,
 * so falling back to one reopens the gate the user was trying to leave. */
export function fallbackAgent(
  personas: { id: string; kind?: string; default?: boolean }[] | null,
  generalAssistant: string,
): string {
  const usable = (personas || []).filter((p) => p.kind !== "workbench" && p.id);
  return usable.find((p) => p.default)?.id || usable[0]?.id || generalAssistant;
}
