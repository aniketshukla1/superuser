import { useEffect, useState } from "react";
import { getBoard, type BoardItem } from "../api";
import { SEC_H } from "../ui";
import { Icon, ICON } from "./Icon";

// The project's work board — what a lead broke down, who has it, and what is
// waiting to be checked.
//
// Read-only, and that is not a shortcut. The board's one real rule is that a
// builder cannot pass its own work, and it holds because the actor on every write
// is the run's session id, which the platform assigns. A person clicking through a
// REST call has no such identity, so every human write would arrive as the same
// anonymous actor — exactly the hole the rule exists to close. You read the board
// here; you change it by asking an assistant.

const ORDER: BoardItem["state"][] = ["review", "in_progress", "blocked", "open", "done", "canceled"];

// Ordered so what needs a person comes first, and named for what the reader has to
// decide rather than for the column's internal id.
const LABEL: Record<BoardItem["state"], string> = {
  review: "Waiting to be checked",
  in_progress: "In progress",
  blocked: "Blocked",
  open: "Not started",
  done: "Done",
  canceled: "Canceled",
};

export function BoardTab({ workspace }: { workspace: string }) {
  const [items, setItems] = useState<BoardItem[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      getBoard(workspace)
        .then((i) => !cancelled && setItems(i))
        .catch(() => !cancelled && setItems([]));
    load();
    // A lead and its workers change the board mid-turn, so a static view goes stale
    // while you are looking at it.
    const t = setInterval(load, 4000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, [workspace]);

  if (items === null) return null;
  if (items.length === 0) {
    return (
      <div className="p-4 text-sm text-muted" data-testid="board-empty">
        Nothing on this project&rsquo;s board yet. A lead assistant puts work here when it breaks
        down into items someone else will pick up.
      </div>
    );
  }

  return (
    <div className="p-4" data-testid="board">
      {ORDER.map((state) => {
        const group = items.filter((i) => i.state === state);
        if (group.length === 0) return null;
        return (
          <div key={state} className="mb-5">
            <div className={`${SEC_H} mb-2`}>
              {LABEL[state]} · {group.length}
            </div>
            {group.map((item) => (
              <BoardRow key={item.id} item={item} />
            ))}
          </div>
        );
      })}
    </div>
  );
}

function BoardRow({ item }: { item: BoardItem }) {
  const [open, setOpen] = useState(false);
  const failed = item.reviews.some((r) => !r.pass);
  const passed = item.reviews.some((r) => r.pass && r.by !== item.builder);

  return (
    <div className="py-2 border-b border-line last:border-0" data-testid="board-item">
      <button className="w-full text-left flex items-start gap-2" onClick={() => setOpen((o) => !o)}>
        <span className="mt-0.5 text-muted">
          <Icon name={open ? "chevronDown" : "chevronRight"} size={ICON.sm} />
        </span>
        <span className="flex-1 min-w-0">
          <span className="block text-md text-ink">{item.title}</span>
          <span className="block text-xs text-muted mt-0.5">
            {item.assignee ? item.assignee : "unassigned"}
            {/* The one state that needs a person to notice: built, and nobody has checked it. */}
            {item.state === "review" && !passed && !failed && " · needs a reviewer who isn’t the builder"}
            {failed && " · a review failed"}
          </span>
        </span>
      </button>

      {open && (
        <div className="pl-6 pt-2 text-sm">
          {item.acceptance ? (
            <div className="text-ink">
              <span className="text-muted">Done when: </span>
              {item.acceptance}
            </div>
          ) : (
            // An item with no criteria can only be reviewed on someone's opinion,
            // which is worth saying rather than leaving blank.
            <div className="text-muted">No acceptance criteria — nothing to check this against.</div>
          )}
          {item.detail && <div className="text-muted mt-1.5 whitespace-pre-wrap">{item.detail}</div>}
          {item.notes.map((n, i) => (
            <div key={i} className="text-muted mt-1.5">
              · {n}
            </div>
          ))}
          {item.reviews.map((r, i) => (
            <div key={i} className="mt-1.5" data-testid="board-review">
              <span className={r.pass ? "text-ink" : "text-red-600"}>
                {r.pass ? "Passed" : "Failed"} by {r.by}
              </span>
              <span className="text-muted"> — {r.evidence}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
