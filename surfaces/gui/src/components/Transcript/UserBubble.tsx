import type { Item } from "../../types";
import { Icon, ICON } from "../Icon";
import { BubbleMeta, ClampedUserText } from "./BubbleMeta";

type UserItem = Extract<Item, { kind: "user" }>;

/** What you said: attachments, the text (folded when long), and its time. */
export function UserBubble({ item, turn, opensExchange }: { item: UserItem; turn: number; opensExchange: boolean }) {
  return (
    <div
      data-turn={turn}
      className={"group self-end max-w-[58%] flex flex-col items-end" + (opensExchange ? " exchange-start" : "")}
    >
      <div className="bubble-user px-3 py-2 rounded-card border border-lineStrong bg-solid text-onSolid whitespace-pre-wrap">
        {item.attachments && item.attachments.length > 0 && (
          <div className="bubble-attachments">
            {item.attachments.map((a, i) =>
              a.kind === "image" ? (
                <img key={i} className="msg-img" src={a.data_url} alt={a.name} />
              ) : (
                <span key={i} className="msg-file">
                  <Icon name="file" size={ICON.sm} className="shrink-0" />
                  {a.name}
                </span>
              ),
            )}
          </div>
        )}
        <ClampedUserText text={item.text} />
      </div>
      <BubbleMeta text={item.text} ts={item.ts} align="right" />
    </div>
  );
}
