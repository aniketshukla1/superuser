import type { ApprovalDecision } from "../../types";
import { TOOL_VERBS, type ApprovalItem } from "./SaveSkillPreview";

export function Buttons({
  item,
  onApprove,
  runTask,
  primaryLabel,
  denyLabel = "Deny",
}: {
  item: ApprovalItem;
  onApprove: (decision: ApprovalDecision) => void;
  runTask?: { id: string; title: string } | null;
  primaryLabel: string;
  denyLabel?: string;
}) {
  const connector = item.category === "connector";
  const offerStanding = !!(runTask && item.standingTarget);
  return (
    <div className="approval-btns">
      <button className="btn approval-primary" onClick={() => onApprove("once")}>
        {primaryLabel}
      </button>
      {offerStanding && (
        <button
          className="btn"
          title={`Always allow ${item.name} → ${item.standingTarget} for “${runTask?.title || "this automation"}” — revoke any time on its Automations page`}
          onClick={() => onApprove("always_task")}
        >
          Allow every time
        </button>
      )}
      {/* In a run context the task-persistent grant replaces the session-scoped one —
          a run session is ephemeral, and two adjacent "always" buttons would blur
          exactly the scope distinction §25 exists to draw. Same rule for run_shell:
          the command-scoped button below is the specific (safer) grant, so the
          tool-wide one stays out of the card. */}
      {/* save_skill and save_persona: no session-wide "always" — every proposal that adds
          something the app will obey later gets its own review (SKILLS-SPEC §5: one gate,
          always). */}
      {!connector && !offerStanding && !["run_shell", "save_skill", "save_persona"].includes(item.name) && (
        <button
          className="btn"
          title={
            item.category === "browser"
              ? `Let it click and type on ${item.args?.site || "this site"} for the rest of this conversation`
              : `Always allow ${TOOL_VERBS[item.name]?.toLowerCase() || item.name} for this session`
          }
          onClick={() => onApprove("always_tool")}
        >
          {item.category === "browser" ? `Always allow on ${item.args?.site || "this site"}` : "Always allow"}
        </button>
      )}
      {item.name === "run_shell" && (
        <button className="btn" onClick={() => onApprove("always_command")}>
          Always allow this command
        </button>
      )}
      <span className="spacer" />
      <button className="btn quiet-deny" onClick={() => onApprove("deny")}>
        {denyLabel}
      </button>
    </div>
  );
}
