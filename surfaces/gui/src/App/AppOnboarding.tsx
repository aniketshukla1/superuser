import { Onboarding } from "../components/Onboarding";
import { getHealth } from "../api";
import type { AppModel } from "./model/useAppModel";

/** First-run onboarding. */
export function AppOnboarding({ m }: { m: AppModel }) {
  const { onboarding, setOnboarding, setModel, loadSettings, openSettings, setSurface,
      startNewSession, openAccess } = m;
  return (
    <>{onboarding && (
        <Onboarding
          onDone={(next) => {
            setOnboarding(false);
            getHealth().then((h) => setModel(h.model)).catch(() => {});
            loadSettings(); // pick up a model connected during setup (clears the composer chip)
            if (next === "gallery") {
              // The specialists tip: land on Settings ▸ Personas, where the Gallery link lives.
              openSettings("personas");
            } else if (next === "automations") {
              // "Create your first automation" (§29) lands on the Automations quickstart.
              setSurface("scheduled");
            } else if (next === "work") {
              // "Start working" teaches by landing (§24, §32): a fresh session with the rail's
              // Access section expanded. Bump after the session switch settles.
              startNewSession();
              setTimeout(openAccess, 80);
            }
          }}
        />
      )}</>
  );
}
