import type { Item } from "../../types";
import type { AssistantItem, TurnItem } from "./ReplyMeta";
import { visualsOf, type ComputerItem, type Visual } from "./visualsOf";

export type Block = { turn: TurnItem[]; live?: boolean } | Visual | { item: Item; i: number };

/** The transcript's items, grouped into turns, visuals and standalone bubbles. */
export function buildBlocks(items: Item[], running?: boolean) {
  // §33 grouping: a turn = the maximal run of assistant/tool/resolved-approval items between
  // breakers (user, connector, notices, plan/dir requests…). Trailing assistant texts are the
  // ANSWER and render as bubbles after the group; interior assistant texts are narration and
  // stay inside. A run with no activity at all is just bubbles (unchanged chat behavior).
  const blocks: Block[] = [];
  let run: TurnItem[] = [];
  // Kept screens seen since the last flush. Not in `run`: they are not steps, and a step
  // row built from one would have no tool to describe.
  let shots: ComputerItem[] = [];
  const flush = (live = false) => {
    if (!run.length && !shots.length) return;
    const turn = [...run];
    const kept = shots;
    run = [];
    shots = [];
    const answers: AssistantItem[] = [];
    // A live run with tool activity keeps its trailing text inside as the status line;
    // a live run with NO activity is a plain streaming reply — bubbles, as ever.
    const keepTrailing = live && turn.some((it) => it.kind !== "assistant");
    if (!keepTrailing)
      while (turn.length && turn[turn.length - 1].kind === "assistant")
        answers.unshift(turn.pop() as AssistantItem);
    if (turn.some((it) => it.kind !== "assistant")) blocks.push({ turn, live });
    else turn.forEach((t) => blocks.push({ item: t, i: -1 }));
    // Between the steps and the answer, because that is the order they happened in and
    // the order the answer reads in: the chart is drawn, then talked about. Outside the
    // step group on purpose — turns render collapsed, and a chart nobody expands is a
    // chart that was never drawn.
    visualsOf(turn).forEach((v) => blocks.push(v));
    kept.forEach((shot) => blocks.push({ shot }));
    answers.forEach((a) => blocks.push({ item: a, i: -1 }));
  };
  items.forEach((item, i) => {
    if (item.kind === "tool" || item.kind === "assistant" || (item.kind === "approval" && item.resolved))
      run.push(item);
    else if (item.kind === "computer") shots.push(item);
    // A request for the user's hands or a value is answered in the composer head, and the
    // step row ("Asked for your hands — …") is its record. Neither breaks the turn.
    else if (item.kind === "computer_help" || item.kind === "computer_secret") return;
    else if (
      // PENDING interactive items render elsewhere (approval/question → composer head) and
      // nothing here — if they broke the run, the trailing narration would flash into an
      // answer bubble exactly while the user is being asked to decide.
      (item.kind === "approval" || item.kind === "dirreq" || item.kind === "planreq" || item.kind === "question") &&
      !item.resolved
    ) {
      return;
    } else {
      flush();
      blocks.push({ item, i });
    }
  });
  flush(!!running);

  const lastTurnIndex = blocks.reduce((acc, b, i) => ("turn" in b ? i : acc), -1);
  return { blocks, lastTurnIndex };
}
