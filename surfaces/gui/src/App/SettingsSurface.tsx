import { SettingsView } from "../components/SettingsView";
import type { AppModel } from "./model/useAppModel";

/** The Settings surface. */
export function SettingsSurface({ m }: { m: AppModel }) {
  const { settingsTab, navigate, openPersona, startNewSession, prefillComposer } = m;
  return (
    <SettingsView
          key={settingsTab}
          initialTab={settingsTab}
          onTabChange={(t) => navigate({ surface: "settings", tab: t }, true)}
          onOpenPersona={(id) => openPersona(id, "settings")}
          onCreateSkill={(description) => {
            // The Skills doorway (SKILLS-SPEC §5.2): creation is a conversation. Fresh
            // session, description in the composer — the user reads and hits send. With
            // no description, the prefill invites them to finish the sentence there.
            startNewSession();
            prefillComposer(
              description
                ? `Build a new skill for me: ${description}`
                : "Build a new skill for me: (describe what the skill should do)",
            );
          }}
        />
  );
}
