import { useState } from "react";
import { PanelHead } from "../IntegrationsView";
import { ModelsTab } from "../ManageTabs";
import { SkillsTab } from "../SkillsTab";
import { BoundariesSection } from "../BoundariesSection";
import { BrowserSection } from "../BrowserSection";
import { showPersonas } from "../../flags";
import { PageTabs } from "../PageTabs";
import { VoiceInputSection } from "./VoiceInputSection";
import { PersonasSection } from "./PersonasSection";
import { AppearanceSection } from "./AppearanceSection";
import { TokenSavingsCard } from "./TokenSavingsCard";
import { CompactionCard } from "./CompactionCard";
import { StallCard } from "./StallCard";

/** A filesystem path that wraps at its slashes. `break-all` split it wherever the line ended,
 *  which put "ai.superuser.ap" on one line and "p/sessions" on the next. */
export function PathText({ path }: { path: string }) {
  const parts = path.split("/");
  return (
    <code className="text-ink" title={path}>
      {parts.map((part, i) => (
        <span key={i}>
          {part}
          {i < parts.length - 1 && (
            <>
              /<wbr />
            </>
          )}
        </span>
      ))}
    </code>
  );
}

// Settings, restructured (Option 2) into a full-page surface that mirrors IntegrationsView's shell:
// a left sub-nav (Appearance · Files · Models · Personas) + centered panel, replacing the old
// top-tab ManageModal. Local/app concerns live here; anything external (Connectors, Messaging, MCP,
// Activity) stays under Integrations. Appearance + Files are re-skinned to the mock's Tailwind idiom;
// Models + Personas host the existing tab components inside the page shell (field re-skin to follow).
// "appearance" is the General tab's stable key — callers deep-link with it, so the
// rename (UX-021) changed only the label. "files" folded into General as a card.
// Exported because it is now part of an address: `#/settings?tab=models` has to
// resolve to one of these, and the router shouldn't be free to invent a sixth.
export type SetTab = "appearance" | "models" | "boundaries" | "browser" | "skills" | "voice" | "personas";

export const INPUT =
  "flex-1 min-w-0 px-3 py-2 rounded-lg border border-line bg-paper text-md text-ink outline-none focus:border-accent";
export const BTN_ACCENT = "ctl bg-accent text-onAccent shrink-0 disabled:opacity-40";
export const BTN_BORDERED =
  "ctl border border-line bg-paper hover:border-lineStrong shrink-0";

// Memories moved out to the Brain surface (`BrainView`) — the memory stores are what
// the product is for, and filing them next to "Voice input" made them look like a
// preference. Settings keeps configuration only.
export const SET_TABS: { key: SetTab; label: string; icon: "sliders" | "code" | "mic" | "sparkle" | "book" | "shield" | "globe" }[] = [
  { key: "appearance", label: "General", icon: "sliders" },
  { key: "models", label: "Models", icon: "code" },
  { key: "boundaries", label: "Boundaries", icon: "shield" },
  { key: "browser", label: "Browser", icon: "globe" },
  { key: "skills", label: "Skills", icon: "book" },
  { key: "voice", label: "Voice input", icon: "mic" },
  { key: "personas", label: "Personas", icon: "sparkle" },
];

export function SettingsView({
  initialTab,
  onTabChange,
  onOpenPersona,
  onCreateSkill,
}: {
  initialTab?: SetTab;
  /** Keeps the address bar honest when a section is picked here rather than
   *  arrived at — otherwise the URL says General while you're reading Models. */
  onTabChange?: (t: SetTab) => void;
  onOpenPersona?: (id: string) => void;
  // Skills doorway (SKILLS-SPEC §5.2): start a new conversation with the description
  // prefilled — the worker builds the skill and proposes it via save_skill.
  onCreateSkill?: (description: string) => void;
}) {
  // Personas is flag-gated (hidden for launch) — filter the tab AND coerce a stale
  // deep-link to it (openSettings("personas") callers) so the page never opens on a
  // section with no nav entry.
  const personas = showPersonas();
  const tabs = personas ? SET_TABS : SET_TABS.filter((t) => t.key !== "personas");
  const wanted = initialTab && (personas || initialTab !== "personas") ? initialTab : "appearance";
  const [tab, setTab] = useState<SetTab>(wanted);

  return (
    <main className="flex-1 min-w-0 flex bg-paper">
      <div className="flex-1 min-w-0 overflow-y-auto hairline-scroll">
        <div className="max-w-4xl mx-auto px-7 py-6">
          <PageTabs
            title="Settings"
            tabs={tabs.map((t) => ({ key: t.key, label: t.label }))}
            current={tab}
            onChange={(k) => {
              setTab(k);
              onTabChange?.(k);
            }}
          >
          {tab === "appearance" ? (
            <AppearanceSection />
          ) : tab === "models" ? (
            <section>
              <PanelHead
                title="Models"
                sub="Providers and the models offered in the composer's picker. Keys are stored only on this computer."
              />
              <ModelsTab />
              {/* Token savings is model-spend behavior, so it lives here (UX-021),
                  not under General. */}
              <div className="mt-6">
                <TokenSavingsCard />
                <CompactionCard />
                <StallCard />
              </div>
            </section>
          ) : tab === "boundaries" ? (
            <BoundariesSection />
          ) : tab === "browser" ? (
            <BrowserSection />
          ) : tab === "skills" ? (
            <SkillsTab onCreateSkill={onCreateSkill} />
          ) : tab === "voice" ? (
            <VoiceInputSection />
          ) : (
            <PersonasSection onOpenPersona={onOpenPersona} />
          )}
          </PageTabs>
        </div>
      </div>
    </main>
  );
}
