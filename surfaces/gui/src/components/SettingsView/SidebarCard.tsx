import { useEffect, useState } from "react";
import { getSettings, setSessionsPeek } from "../../api";
import { reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";

export function SidebarCard() {
  const [peek, setPeek] = useState<number | null>(null);

  useEffect(() => {
    getSettings()
      .then((s) => setPeek(s.sessions_peek || 5))
      .catch(() => setPeek(5));
  }, []);

  const save = async (n: number) => {
    const before = peek;
    const clamped = Math.max(1, Math.min(n || 5, 50));
    setPeek(clamped);
    const saved = await reportIfFailed(setSessionsPeek(clamped), "Couldn't save how many chats to show.");
    if (!saved) setPeek(before);
  };

  if (peek === null) return null;
  return (
    <div className={CARD + " p-4 mb-4"}>
      <div className={FIELD_LABEL}>Sidebar</div>
      <label className="flex items-center gap-3 mt-2.5">
        <span className="text-md text-ink">Conversations shown per assistant</span>
        <input
          type="number"
          min={1}
          max={50}
          value={peek}
          className="w-16 px-2 py-1.5 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent"
          onChange={(e) => save(Number(e.target.value))}
        />
      </label>
      <div className={FIELD_HELP}>
        Longer lists collapse behind &ldquo;Show more&rdquo;. Applies per assistant and per project.
      </div>
    </div>
  );
}
