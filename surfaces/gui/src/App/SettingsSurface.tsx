import { SettingsView } from "../components/SettingsView";
import type { AppModel } from "./model/useAppModel";

/** What the composer is given when someone starts a skill from the Skills page. */
const skillRequest = (description?: string) =>
  `Build a new skill for me: ${description || "(describe what the skill should do)"}`;

/** The Settings surface. */
export function SettingsSurface({ m }: { m: AppModel }) {
  // The Skills page's way in (SKILLS-SPEC §5.2): a skill is made by talking. A new session
  // opens with the description in the composer for the user to read and send; with no
  // description, the text there asks them to finish the sentence.
  const createSkill = (description?: string) => {
    m.startNewSession();
    m.prefillComposer(skillRequest(description));
  };
  return (
    <SettingsView
      key={m.settingsTab}
      initialTab={m.settingsTab}
      onTabChange={(tab) => m.navigate({ surface: "settings", tab }, true)}
      onOpenPersona={(id) => m.openPersona(id, "settings")}
      onCreateSkill={createSkill}
    />
  );
}
