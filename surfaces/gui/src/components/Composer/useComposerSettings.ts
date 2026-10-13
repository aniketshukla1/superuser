import { useEffect, useState } from "react";
import { getSettings } from "../../api";

/** What the composer's chips need from settings: the effort scale and provider prefixes. */
export function useComposerSettings() {
  // Attach-time PDF thresholds (Settings → Token savings): a PDF over the user's page or
  // size limit is REJECTED with a visible notice — never attached, never silently dropped.
  // The rationale is token cost: a big PDF re-rides every turn of the conversation.
  // The effort scale comes from the server with the settings, not from a constant here:
  // a client that knows the levels is a client that disagrees with the server the first
  // time they change. Absent (older backend) → the chip does not render at all.
  const [effort, setEffort] = useState<{ value: string; levels: string[] } | null>(null);
  const [providers, setProviders] = useState<string[] | undefined>(undefined);
  useEffect(() => {
    let live = true;
    void getSettings()
      .then((s) => {
        if (!live) return;
        setProviders(s.provider_prefixes);
        if (!s.effort || !s.effort_levels?.length) return;
        setEffort({ value: s.effort, levels: s.effort_levels });
      })
      .catch(() => {
        /* offline: the chip stays hidden rather than guessing a scale */
      });
    return () => {
      live = false;
    };
  }, []);

  return { effort, setEffort, providers };
}
