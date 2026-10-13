import { useEffect, useState } from "react";
import { createAutomation, deleteAutomation, getAutomations, announceAutomationsChanged, getSettings, type Automation } from "../../api";
import { Icon, ICON } from "../Icon";
import { PanelHead } from "../IntegrationsView";
import { AutomationQuickstart } from "../AutomationQuickstart";
import { reportIfFailed } from "../../notify";
import { CARD } from "../../ui";
import { exactTime } from "../../when";
import { lastRunText, plural, scheduleLabel } from "../../scheduleText";
import { Shell, fmt, type Props } from "./Props";
import { NewAutomationForm } from "./NewAutomationForm";
import { TaskDetail } from "./TaskDetail";

export function ScheduledView({ onOpenRun, onRunNow, initialOpenId, newRequest, frame = (c) => c }: Props) {
  const [tasks, setTasks] = useState<Automation[]>([]);
  // The models this machine can actually run, for the per-automation picker. Fetched
  // here rather than threaded down from App: this surface is opened on its own and
  // the list is small, static and cheap.
  const [models, setModels] = useState<string[]>([]);
  const [modelLabels, setModelLabels] = useState<Record<string, string>>({});
  useEffect(() => {
    getSettings()
      .then((s) => {
        setModels(s.models || []);
        setModelLabels(s.model_labels || {});
      })
      .catch(() => {});
  }, []);
  const [openId, setOpenId] = useState<string | null>(initialOpenId ?? null);
  const [showForm, setShowForm] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  // Which card's trash has been armed. Cleared on any other interaction with the list.
  const [confirmDelId, setConfirmDelId] = useState<string | null>(null);

  // The sidebar's Scheduled band can retarget an ALREADY-open Automations surface —
  // initial state alone would ignore the change (UX-023).
  useEffect(() => {
    if (initialOpenId) setOpenId(initialOpenId);
  }, [initialOpenId]);

  // The sidebar's "+" lands here: show the form, and close any open task detail so
  // the new one is what you are looking at.
  useEffect(() => {
    if (newRequest) {
      setOpenId(null);
      setShowForm(true);
    }
  }, [newRequest]);

  const refresh = () => getAutomations().then(setTasks).catch(() => setTasks([]));
  useEffect(() => {
    refresh();
    const h = setInterval(refresh, 5000);
    return () => clearInterval(h);
  }, []);

  // Create from a payload, refresh the list, and open the new task's detail. `permissions`
  // rides through for quickstart recipes (§25 write grants).
  const create = async (payload: {
    title: string;
    instructions: string;
    cron?: string;
    permissions?: { tool: string; target: string; access: "read" | "write" }[];
  }) => {
    setBusy(payload.title);
    try {
      const res = await createAutomation(payload);
      announceAutomationsChanged(); // new entry shows in the sidebar band right away
      await refresh();
      if (res.ok && res.task) {
        setShowForm(false);
        setOpenId(res.task.id);
      } else if (res.error) {
        alert(res.error);
      }
    } finally {
      setBusy(null);
    }
  };

  if (openId) {
    return (
      <TaskDetail
        models={models}
        modelLabels={modelLabels}
        id={openId}
        onBack={() => { setOpenId(null); refresh(); }}
        onOpenRun={onOpenRun}
        onRunNow={onRunNow}
      />
    );
  }

  const empty = tasks.length === 0;

  return (
    <Shell frame={frame}>
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0">
          <PanelHead title="Scheduled" sub="Recurring tasks Superuser runs at a set time." />
        </div>
        <button className="btn shrink-0" onClick={() => setShowForm((v) => !v)} aria-expanded={showForm}>
          <Icon name="plus" size={ICON.md} />
          New automation
        </button>
      </div>

      <div className="text-sm text-faint flex items-center gap-1.5 mb-4">
        <Icon name="clock" size={ICON.sm} className="shrink-0" />
        <span>
          Runs only while Superuser is open — a missed schedule catches up once when it next
          starts.
        </span>
      </div>

      {showForm && (
        <NewAutomationForm
          busy={busy !== null}
          onCancel={() => setShowForm(false)}
          onCreate={create}
        />
      )}

      {/* The quickstart (§29): ONE template system — role recipes + generic templates, each
          card with §27 connector dots; picking one expands the configure card. */}
      {(empty || showForm) && <AutomationQuickstart busy={busy !== null} onCreate={create} />}

      {empty ? (
        !showForm && (
          <div className={CARD + " p-4 text-sm text-muted"}>
            No scheduled tasks yet — use a template above, click <strong>+ New automation</strong>,
            or just ask Superuser in a session.
          </div>
        )
      ) : (
        <div className="flex flex-col gap-2.5">
          {tasks.map((t) => (
            <div
              className={CARD + " sched-card px-4 py-3 cursor-pointer hover:border-lineStrong transition-colors"}
              key={t.id}
              onClick={() => {
                setConfirmDelId(null); // an armed delete does not survive looking elsewhere
                setOpenId(t.id);
              }}
            >
              <div className="flex items-center justify-between gap-2.5 mb-1">
                <span className="text-md font-semibold truncate">{t.title}</span>
                {/* Two clicks, like a session's delete (Sidebar.tsx). One click used to
                    remove an automation outright — a thing that sends messages and
                    spends tokens on its own — with no confirmation and no undo
                    (UX-REPORT §9). */}
                <button
                  className={"sched-card-del" + (confirmDelId === t.id ? " confirming" : "")}
                  title={confirmDelId === t.id ? "Click again to delete" : "Delete automation"}
                  aria-label={confirmDelId === t.id ? `Confirm deleting ${t.title}` : `Delete ${t.title}`}
                  data-testid="sched-card-del"
                  onClick={async (e) => {
                    e.stopPropagation();
                    if (confirmDelId !== t.id) {
                      setConfirmDelId(t.id);
                      return;
                    }
                    setConfirmDelId(null);
                    await reportIfFailed(deleteAutomation(t.id), `Couldn't delete “${t.title}”.`);
                    refresh();
                  }}
                >
                  {confirmDelId === t.id ? (
                    <span className="text-xs font-medium">Delete?</span>
                  ) : (
                    <Icon name="trash" size={ICON.md} />
                  )}
                </button>
              </div>
              <div className="flex items-center gap-1.5 text-sm text-muted">
                <Icon name="clock" size={ICON.md} className="text-faint shrink-0" />
                {t.enabled ? scheduleLabel(t.schedule) : "Paused"}
                {t.enabled && t.next_run ? <span title={exactTime(t.next_run)}> · next {fmt(t.next_run)}</span> : null} ·{" "}
                {plural(t.run_count, "run")}
                {t.last_status ? (
                  <>
                    {" · "}
                    <span className={"run-" + t.last_status}>{lastRunText(t.last_status)}</span>
                  </>
                ) : null}
              </div>
            </div>
          ))}
        </div>
      )}
    </Shell>
  );
}
