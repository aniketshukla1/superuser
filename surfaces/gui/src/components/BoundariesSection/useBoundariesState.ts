import { useEffect, useRef, useState } from "react";
import { getBoundaries, type Boundaries, type BoundariesState, type BoundaryProblem, type BoundaryReport } from "../../api";
import { EMPTY } from "./INPUT";

/** The boundaries page's state: what is saved, the draft, and what checking found. */
export function useBoundariesState() {
  const [state, setState] = useState<BoundariesState | null>(null);
  const [draft, setDraft] = useState<Boundaries | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  // Problems the server reported on the last save, by list and position.
  const [problems, setProblems] = useState<BoundaryProblem[]>([]);
  // What checking each deny rule found while it was being typed. null = reads fine.
  const [checks, setChecks] = useState<Record<number, string | null>>({});
  const [report, setReport] = useState<BoundaryReport | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  // The allow list as typed. Kept apart from the draft so a new, still-empty line
  // survives until something is written on it.
  const [allowText, setAllowText] = useState("");
  const timers = useRef<Record<number, ReturnType<typeof setTimeout>>>({});

  const load = () =>
    getBoundaries()
      .then((s) => {
        if (!s) {
          setLoadFailed(true);
          return;
        }
        setLoadFailed(false);
        setState(s);
        // An unreadable file loads as a single "refuse everything" rule. That rule is
        // the server's backstop, not something the user wrote, so it never becomes
        // the draft — saving it would make the backstop permanent.
        const start = s.problem ? EMPTY : s.boundaries;
        setDraft(start);
        setAllowText(start.allow.join("\n"));
        setProblems(s.problems);
        setChecks({});
        setReport(null);
      })
      .catch(() => setLoadFailed(true));

  useEffect(() => {
    void load();
    return () => Object.values(timers.current).forEach(clearTimeout);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    state,
    setState,
    draft,
    setDraft,
    loadFailed,
    problems,
    setProblems,
    checks,
    setChecks,
    report,
    setReport,
    testing,
    setTesting,
    saving,
    setSaving,
    saved,
    setSaved,
    allowText,
    setAllowText,
    timers,
    load,
  };
}
