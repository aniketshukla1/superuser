export type Notice = { name: string; text: string; tone: "ok" | "warn" };

/** The state-change callout (SKILLS-SPEC §4.1 #2): name-first so the user knows WHICH skill,
 * and visually distinct so it can't be skimmed past (tester ask 2026-07-27). */
export function SkillNotice({ notice, onDismiss }: { notice: Notice; onDismiss: () => void }) {
  return (
    <div
      role="status"
      className={
        "mb-3 flex items-start gap-2 rounded-lg border px-3 py-2 text-sm " +
        (notice.tone === "ok"
          ? "bg-tealSoft/70 text-tealInk border-tealInk/20"
          : "bg-warnSoft/70 text-warnInk border-warnInk/20")
      }
    >
      <span className="min-w-0">
        <b>{notice.name}</b> {notice.text}
      </span>
      <button
        className="ml-auto shrink-0 opacity-60 hover:opacity-100"
        aria-label="Dismiss"
        onClick={onDismiss}
      >
        ✕
      </button>
    </div>
  );
}
