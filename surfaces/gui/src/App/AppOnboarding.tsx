import { getHealth } from "../api";
import { Onboarding } from "../components/Onboarding";
import type { AppModel } from "./model/useAppModel";

/** The first-run setup, and where each of its endings leads. */
export function AppOnboarding({ m }: { m: AppModel }) {
  if (!m.onboarding) return null;

  const finish = (next?: string) => {
    m.setOnboarding(false);
    getHealth()
      .then((health) => m.setModel(health.model))
      .catch(() => {});
    // A model connected during setup clears the composer's "No model connected" chip.
    m.loadSettings();
    // The specialists tip goes to Settings ▸ Personas, where the Gallery link is.
    if (next === "gallery") m.openSettings("personas");
    // "Create your first automation" (§29) goes to the Automations quickstart.
    else if (next === "automations") m.setSurface("scheduled");
    // "Start working" teaches by arriving (§24, §32): a fresh session with the panel's Access
    // section open, bumped once the switch to the new session has settled.
    else if (next === "work") {
      m.startNewSession();
      setTimeout(m.openAccess, 80);
    }
  };

  return <Onboarding onDone={finish} />;
}
