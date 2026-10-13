/** Step 3: set up — two good ways to start. */
export function StepDone({ finish }: { finish: (next?: "work" | "gallery" | "automations") => void }) {
  return (
    <section data-testid="ob-step-done" className="flex-1 min-h-0 flex flex-col overflow-y-auto">
      <div className="text-center">
        <div className="w-12 h-12 rounded-full bg-okSoft text-ok grid place-items-center mx-auto mb-3 text-xl">
          ✓
        </div>
        <h1 className="text-xl font-semibold mb-1">You're set up</h1>
        <p className="text-md text-muted mb-5">Two good ways to start:</p>
      </div>

      <button
        className="w-full flex items-start gap-3 rounded-xl2 border border-line hover:border-accent bg-panel px-4 py-3.5"
        onClick={() => finish("automations")}
        data-testid="ob-cta-automation"
      >
        <span className="w-9 h-9 rounded-lg bg-accentSoft text-accent grid place-items-center text-lg shrink-0">
          ◷
        </span>
        <span className="flex-1 min-w-0 text-left">
          <b className="block text-md">Create your first automation</b>
          <span className="text-sm text-muted">
            A weekly digest, a morning brief — pick a template, running in two minutes.
          </span>
        </span>
        <span className="text-faint self-center">›</span>
      </button>
      <button
        className="w-full flex items-start gap-3 rounded-xl2 border border-line hover:border-accent bg-panel px-4 py-3.5 mt-2.5"
        onClick={() => finish("work")}
        data-testid="ob-start"
      >
        <span className="w-9 h-9 rounded-lg bg-accentSoft text-accent grid place-items-center text-lg shrink-0">
          ✦
        </span>
        <span className="flex-1 min-w-0 text-left">
          <b className="block text-md">Start working with Assistant</b>
          <span className="text-sm text-muted">
            Open a session and just ask — analyze files, draft, research, build.
          </span>
        </span>
        <span className="text-faint self-center">›</span>
      </button>

      {/* The Specialist-assistants gallery card and the per-session-scope line stay HIDDEN
          (owner call 2026-07-12); the finish("gallery") plumbing remains for their return. */}

      <p className="text-xs text-faint text-center mt-auto pt-5">
        Replay this setup anytime: Settings ▸ Appearance ▸ Run setup again.
      </p>
    </section>
  );
}
