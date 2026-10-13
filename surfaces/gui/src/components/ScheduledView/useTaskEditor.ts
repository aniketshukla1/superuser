import { useState } from "react";
import { updateAutomation, type Automation } from "../../api";
import { reportIfFailed } from "../../notify";
import { fromCron, toCron } from "./Props";

/** The automation editor's draft: title, instructions, schedule and model, and its save. */
export function useTaskEditor(id: string, refresh: () => Promise<unknown>) {
  const [editing, setEditing] = useState(false);
  // "" is Default — run on whatever the app would pick. A real option, not an absent
  // one, which is why the PATCH sends `model: ""` rather than omitting the key.
  const [model, setModel] = useState("");
  const [title, setTitle] = useState("");
  const [instructions, setInstructions] = useState("");
  const [time, setTime] = useState("09:00");
  const [freq, setFreq] = useState("daily");
  const [saving, setSaving] = useState(false);

  const beginEdit = (task: Automation) => {
    setTitle(task.title);
    setInstructions(task.instructions);
    const { time: t, freq: f } = fromCron(task.schedule_raw?.cron);
    setTime(t);
    setFreq(f);
    setModel(task.model || "");
    setEditing(true);
  };
  const saveEdit = async () => {
    setSaving(true);
    try {
      const saved = await reportIfFailed(
        updateAutomation(id, {
          title: title.trim(),
          instructions: instructions.trim(),
          cron: toCron(time, freq),
          // Always sent, including "" — that is how you put an automation back on the
          // app's default, and omitting the key would make that unreachable.
          model,
        }),
        "Couldn't save this automation.",
      );
      await refresh();
      // Staying in the editor keeps the user's typing when the save didn't land.
      if (saved) setEditing(false);
    } finally {
      setSaving(false);
    }
  };

  return {
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
  };
}
