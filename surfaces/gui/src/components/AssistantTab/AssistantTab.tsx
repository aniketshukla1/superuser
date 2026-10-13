import { useEffect, useState } from "react";
import { deletePersona, getAutomations, getPersonas, listSkills, type ArtifactInfo, type Automation, type Persona, type SkillRow } from "../../api";
import { Icon, ICON, AVATAR } from "../Icon";
import { PersonaGlyph } from "../personaIcon";
import { GuidedAdd, Section } from "./GuidedAdd";
import { InstructionsSection } from "./InstructionsSection";

export function AssistantTab({
  sessionId,
  personaId,
  artifacts,
  onSend,
  onViewFiles,
}: {
  sessionId: string;
  personaId?: string;
  artifacts: ArtifactInfo[];
  /** Sends a message as the user, in this conversation. */
  onSend: (text: string) => void;
  onViewFiles: () => void;
}) {
  const [persona, setPersona] = useState<Persona | null>(null);
  const [tasks, setTasks] = useState<Automation[]>([]);
  const [skills, setSkills] = useState<SkillRow[]>([]);
  const [adding, setAdding] = useState<"" | "workflow" | "schedule">("");

  const [confirmRemove, setConfirmRemove] = useState(false);

  useEffect(() => {
    let dead = false;
    getPersonas()
      .then((ps) => !dead && setPersona(ps.find((p) => p.id === personaId) || null))
      .catch(() => {});
    getAutomations()
      .then((a) => !dead && setTasks(a.filter((t) => !personaId || t.agent === personaId)))
      .catch(() => {});
    listSkills()
      .then((s) => !dead && setSkills(s.filter((x) => x.enabled)))
      .catch(() => {});
    return () => {
      dead = true;
    };
  }, [personaId]);

  useEffect(() => {
    setConfirmRemove(false);
  }, [sessionId]);

  const who = persona?.name || "this assistant";
  const recent = artifacts.slice(0, 5);

  return (
    <div className="asst-tab">
      <div className="asst-hero">
        <PersonaGlyph icon={persona?.icon || "sparkles"} family={persona?.family} size={AVATAR.sm} />
        <div className="asst-hero-text">
          <div className="asst-name">{who}</div>
          {persona?.tagline && <div className="asst-tagline">{persona.tagline}</div>}
        </div>
      </div>

      <InstructionsSection sessionId={sessionId} />

      {/* "Skills" everywhere: the composer's "/" list, the + menu and the Settings tab are
          this same set, and it was called Workflows only here. */}
      <Section
        title="Skills"
        action={adding === "workflow" ? undefined : "+ Add skill"}
        onAction={() => setAdding("workflow")}
      >
        {adding === "workflow" ? (
          <GuidedAdd
            question={`What should the workflow help ${who} do? I can turn it into a repeatable process with inputs, steps, outputs and review checkpoints.`}
            chips={["A weekly summary", "A monthly review", "A daily brief"]}
            freeform={`Help me build a repeatable workflow for ${who}. Ask me what it should do, then turn it into a process with inputs, steps, outputs and review checkpoints.`}
            onSend={(t) => {
              setAdding("");
              onSend(
                `Set up a repeatable workflow for ${who}: ${t}. Ask me anything you need, then write it down as a skill with inputs, steps, outputs and review checkpoints.`,
              );
            }}
            onCancel={() => setAdding("")}
          />
        ) : skills.length === 0 ? (
          <div className="rail-muted">No skills yet.</div>
        ) : (
          <div className="asst-list">
            {skills.slice(0, 6).map((s) => (
              <button key={s.name} className="asst-row" onClick={() => onSend(`Run the ${s.name} workflow.`)}>
                <Icon name="sparkle" size={ICON.md} />
                <span className="asst-row-name">{s.name}</span>
                {s.scope !== "global" && <span className="asst-row-meta">{s.scope}</span>}
              </button>
            ))}
          </div>
        )}
      </Section>

      <Section
        title="Scheduled"
        action={adding === "schedule" ? undefined : "+ Add schedule"}
        onAction={() => setAdding("schedule")}
      >
        {adding === "schedule" ? (
          <GuidedAdd
            question={`What should ${who} do on a schedule, and how often? I'll set it up and show you the first run before it goes out.`}
            chips={["Every weekday morning", "Every Monday", "On the 1st of the month"]}
            freeform={`Help me put something on a schedule for ${who}. Ask me what it should do and how often, then set it up.`}
            onSend={(t) => {
              setAdding("");
              onSend(`Schedule something for ${who} to run ${t.toLowerCase()}. Ask me what it should do, then set it up.`);
            }}
            onCancel={() => setAdding("")}
          />
        ) : tasks.length === 0 ? (
          <div className="rail-muted">Nothing scheduled.</div>
        ) : (
          <div className="asst-list">
            {tasks.map((t) => (
              <div key={t.id} className="asst-row static">
                <Icon name="clock" size={ICON.md} />
                <span className="asst-row-name">{t.title}</span>
                <span className="asst-row-meta">{t.enabled ? t.schedule : "paused"}</span>
              </div>
            ))}
          </div>
        )}
      </Section>

      <Section title="Recent files" action={artifacts.length ? "View all" : undefined} onAction={onViewFiles}>
        {recent.length === 0 ? (
          <div className="rail-muted">Nothing produced yet.</div>
        ) : (
          <div className="asst-list">
            {recent.map((a) => (
              <button key={a.path} className="asst-row" onClick={onViewFiles}>
                <Icon name="file" size={ICON.md} />
                <span className="asst-row-name">{a.name}</span>
              </button>
            ))}
          </div>
        )}
      </Section>

      <Section title="Manage assistant">
        {persona?.builtin ? (
          <div className="rail-muted">{who} is built in and can't be removed.</div>
        ) : !persona ? (
          <div className="rail-muted">No assistant selected.</div>
        ) : confirmRemove ? (
          <div className="asst-confirm">
            <div className="rail-muted">
              Remove {who}? This deletes the assistant. Your conversations and files stay.
            </div>
            <div className="asst-guided-foot">
              <button
                className="artifact-text-btn danger"
                onClick={async () => {
                  await deletePersona(persona.id);
                  setConfirmRemove(false);
                }}
              >
                Remove
              </button>
              <button className="artifact-text-btn" onClick={() => setConfirmRemove(false)}>
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <button className="asst-link danger" onClick={() => setConfirmRemove(true)}>
            Remove this assistant
          </button>
        )}
      </Section>
    </div>
  );
}
