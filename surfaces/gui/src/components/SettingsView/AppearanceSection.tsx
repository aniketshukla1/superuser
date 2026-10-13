import { useEffect, useState } from "react";
import { setOnboarded } from "../../api";
import { getAutostart, getKeepAwake, isTauri, setAutostart, setKeepAwake } from "../../tauri";
import { ThemePicker } from "../ThemePicker";
import { AccentPicker } from "../AccentPicker";
import { PanelHead } from "../IntegrationsView";
import { EmailBotSection } from "../EmailBotSection";
import { ExpensesSection } from "../ExpensesSection";
import { ChiefOfStaffSection } from "../ChiefOfStaffSection";
import { BugReproSection } from "../BugReproSection";
import { PerfSection } from "../PerfSection";
import { TelegramSection } from "../TelegramSection";
import { reportIfFailed } from "../../notify";
import { CARD, FIELD_HELP, FIELD_LABEL } from "../../ui";
import { BTN_BORDERED } from "./SettingsView";
import { TrustedWorkspacesCard, UpdateInline } from "./TrustedWorkspacesCard";
import { GenerativeUiCard } from "./StallCard";
import { ApprovalsCard, TeamCard } from "./ApprovalsCard";
import { PrivacyCard, RiskOverridesCard } from "./RiskOverridesCard";
import { SidebarCard } from "./SidebarCard";
import { FilesCard } from "./FilesCard";

// -- Appearance + app behaviour ------------------------------------------------
export function AppearanceSection() {
  const [autostart, setAuto] = useState(false);
  const [keepAwake, setKeep] = useState(false);
  const desktop = isTauri();

  useEffect(() => {
    if (isTauri()) {
      getAutostart().then((v) => setAuto(!!v));
      getKeepAwake().then((v) => setKeep(!!v));
    }
  }, []);

  const toggleAuto = async (v: boolean) => setAuto(!!(await setAutostart(v)));
  const toggleKeep = async (v: boolean) => setKeep(!!(await setKeepAwake(v)));
  const runSetupAgain = async () => {
    const saved = await reportIfFailed(setOnboarded(false), "Couldn't restart the setup guide.");
    if (!saved) return;
    window.dispatchEvent(new CustomEvent("assistant:open-onboarding"));
  };

  return (
    <section>
      <PanelHead title="General" sub="How Superuser looks and behaves on this machine." />

      <div className={CARD + " p-4 mb-4"}>
        <div className={FIELD_LABEL}>Theme</div>
        <div className={FIELD_HELP + " mb-3"}>Auto follows your Mac&rsquo;s appearance, with Carbon for dark and Porcelain for light.</div>
        <ThemePicker />
      </div>

      <div className={CARD + " p-4 mb-4"}>
        <AccentPicker />
      </div>

      <PrivacyCard />

      <SidebarCard />

      <GenerativeUiCard />

      <FilesCard />

      <ApprovalsCard />

      <TrustedWorkspacesCard />

      <TeamCard />

      <RiskOverridesCard />

      {desktop && (
        <div className={CARD + " p-4"}>
          <div className={FIELD_LABEL + " mb-2.5"}>Always-on</div>
          <label className="flex items-start gap-3 py-2">
            <input type="checkbox" className="mt-0.5" checked={autostart} onChange={(e) => toggleAuto(e.target.checked)} />
            <span>
              <span className="block text-md text-ink">Open at login</span>
              <span className="block text-sm text-muted">Launch Superuser automatically when you sign in.</span>
            </span>
          </label>
          <label className="flex items-start gap-3 py-2">
            <input type="checkbox" className="mt-0.5" checked={keepAwake} onChange={(e) => toggleKeep(e.target.checked)} />
            <span>
              <span className="block text-md text-ink">Keep this system awake</span>
              <span className="block text-sm text-muted">Prevent idle sleep so scheduled tasks fire on time.</span>
            </span>
          </label>
        </div>
      )}

      {/* One card for the app-lifecycle actions (UX-021): the onboarding replay (§24 —
          every build, the browser dev shell runs the same first-run flow) and, on
          desktop, the manual update check (launch also checks automatically). */}
      <div className={CARD + " p-4 mt-4"}>
        <div className={FIELD_LABEL + " mb-2"}>Setup &amp; updates</div>
        <div className="flex items-center gap-2">
          <button className={BTN_BORDERED} onClick={runSetupAgain}>
            Run setup again
          </button>
          {desktop && <UpdateInline />}
        </div>
        <div className={FIELD_HELP}>Replays the first-run setup: model, first automation, tips.</div>
      </div>

      <TelegramSection />
      <EmailBotSection />
      <ExpensesSection />
      <ChiefOfStaffSection />
      <BugReproSection />
      <PerfSection />
    </section>
  );
}
