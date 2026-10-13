import { useEffect, useState } from "react";
import { deleteAutomation, getAutomation, markAutomationSeen, announceAutomationsChanged, updateAutomation, type Automation, type AutomationRun } from "../../api";
import { reportIfFailed } from "../../notify";
import { exactTime } from "../../when";
import { scheduleLabel } from "../../scheduleText";
import { Toggle } from "../Toggle";
import { Shell, fmt } from "./Props";
import { useTaskEditor } from "./useTaskEditor";
import { DetailActions } from "./DetailActions";
import { RunList } from "./RunList";
import { ScheduleEditor } from "./ScheduleEditor";
import { StandingGrants } from "./StandingGrants";

export function TaskDetail({
  id,
  onBack,
  onOpenRun,
  onRunNow,
  models,
  modelLabels,
}: {
  id: string;
  onBack: () => void;
  models: string[];
  modelLabels: Record<string, string>;
  onOpenRun: (
    sessionId: string,
    workspace: string,
    agent: string,
    task?: { id: string; title: string },
  ) => void;
  onRunNow: (taskId: string, title?: string) => void;
}) {
  const [task, setTask] = useState<Automation | null>(null);
  const [runs, setRuns] = useState<AutomationRun[]>([]);
  // Delete asks twice here as it does on the list card: this used to delete on one click from
  // the detail page while the same action on the card beside it asked first.
  const [confirmDelete, setConfirmDelete] = useState(false);

  // The seen mark AS OF opening — the "new" pills compare against this frozen value
  // while mark-seen advances the stored one (badge clears; highlights survive).
  const [seenMark, setSeenMark] = useState<number | null>(null);

  const refresh = () =>
    getAutomation(id)
      .then((d) => {
        if (!d.task) {
          // Deleted (or a stale reopen target): "Loading…" forever is a trap —
          // fall back to the overview (owner-hit 2026-07-20).
          onBack();
          return;
        }
        setTask(d.task);
        setRuns(d.runs || []);
        setSeenMark((cur) => (cur === null ? d.task?.seen_runs_at ?? 0 : cur));
      })
      .catch(() => {});
  const {
    editing,
    setEditing,
    model,
    setModel,
    title,
    setTitle,
    instructions,
    setInstructions,
    time,
    setTime,
    freq,
    setFreq,
    saving,
    beginEdit,
    saveEdit,
  } = useTaskEditor(id, refresh);
  useEffect(() => {
    setSeenMark(null);
    // Read the mark before advancing it. `refresh` is what captures the pre-open
    // `seen_runs_at`, and marking seen overwrites that same field — fired
    // concurrently, the POST sometimes landed first, so the GET came back with the
    // *advanced* mark, `seenMark` froze at "now", and no run was newer than it.
    // The "new" pills then silently didn't render on exactly the opens they exist
    // for. Sequencing is the whole fix; the badge clears a beat later, which is
    // also the more honest moment — once the detail is actually on screen.
    refresh().then(() => {
      // Opening the detail IS reading it: advance the seen mark and nudge the
      // sidebar so the badge clears (UX-023).
      markAutomationSeen(id)
        .then(() => announceAutomationsChanged())
        .catch(() => {});
    });
  }, [id]);

  if (!task)
    return (
      <Shell>
        <div className="text-md text-muted">Loading…</div>
      </Shell>
    );

  const startEdit = () => beginEdit(task);
  const toggle = async () => {
    // Without this the switch just sprang back on a refused write and nothing said
    // why — indistinguishable from a control that doesn't work (UX-REPORT §5).
    await reportIfFailed(
      updateAutomation(id, { enabled: !task.enabled }),
      task.enabled ? "Couldn't pause this automation." : "Couldn't resume this automation.",
    );
    refresh();
  };
  const remove = async () => {
    const saved = await reportIfFailed(deleteAutomation(id), "Couldn't delete this automation.");
    if (!saved) return; // it still exists; stay on it
    announceAutomationsChanged(); // the sidebar band must not wait out its poll
    onBack();
  };

  return (
    <Shell>
      <button className="text-md text-muted hover:text-ink mb-3" onClick={onBack}>
        ← Automations
      </button>
      <div className="sched-detail">
        <div className="sched-detail-head">
          {editing ? (
            <input
              className="tmpl-input sched-edit-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Title"
            />
          ) : (
            <h2 className="text-lg font-semibold tracking-tight">{task.title}</h2>
          )}
          <DetailActions
            title={task.title}
            editing={editing}
            saving={saving}
            canSave={!!title.trim() && !!instructions.trim()}
            onSave={saveEdit}
            onCancelEdit={() => setEditing(false)}
            confirmDelete={confirmDelete}
            setConfirmDelete={setConfirmDelete}
            onRemove={remove}
            onRunNow={() => onRunNow(id, task.title)}
            onEdit={startEdit}
          />
        </div>

        {editing ? (
          <ScheduleEditor
            time={time}
            setTime={setTime}
            freq={freq}
            setFreq={setFreq}
            model={model}
            setModel={setModel}
            models={models}
            modelLabels={modelLabels}
            pinned={task.model}
          />
        ) : (
          // The switch is the page's one standing control, so it gets its own row with its state
          // written beside it, instead of sitting inside a sentence of metadata.
          <div className="sched-state">
            <Toggle
              checked={task.enabled}
              onChange={toggle}
              label={task.enabled ? "Pause this automation" : "Resume this automation"}
              testId="task-enabled"
            />
            <span className="sched-state-label">{task.enabled ? "Active" : "Paused"}</span>
            <span className="sched-state-meta">
              {scheduleLabel(task.schedule)}
              {task.enabled && task.next_run ? (
                <span title={exactTime(task.next_run)}> · next run {fmt(task.next_run)}</span>
              ) : null}
              {task.model && <> · {modelLabels[task.model] || task.model}</>}
            </span>
          </div>
        )}

        <div className="sa-sub">Instructions</div>
        {editing ? (
          <textarea
            className="tmpl-input tmpl-textarea sched-edit-instr"
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
          />
        ) : (
          <div className="sched-instructions">{task.instructions}</div>
        )}

        <StandingGrants
          rules={task.always_allowed || []}
          onRevoke={async (entry) => {
            await updateAutomation(id, { revoke: entry });
            refresh();
          }}
        />

        <RunList runs={runs} seenMark={seenMark} task={task} onOpenRun={onOpenRun} />
      </div>
    </Shell>
  );
}
