import { useState } from "react";
import { type IconName } from "./Icon";
import { PanelHead } from "./IntegrationsView";
import { PageTabs } from "./PageTabs";
import { JournalSection } from "./JournalSection";
import { KnowledgeSection } from "./KnowledgeSection";
import { MemoriesTab } from "./MemoriesTab";
import { ProfileSection } from "./ProfileSection";

// The Brain — a first-class surface, not a tab inside Settings.
//
// All three memory stores were reachable only through Settings ▸ Memories, stacked
// in one scroll: the thing the product is named for, filed next to "Voice input".
// This gives them a destination of their own and separates them, because they
// answer different questions and are written by different things:
//
//   Memories  — organized by source, written by ingestion ("what came in from GitHub?")
//   Timeline  — organized by time, written automatically ("what did we establish, and when?")
//   Knowledge — organized by topic, written deliberately ("what do I know about Postgres?")
//   Profile   — organized by facet, learned from you ("how does this person want to work?")
//
// The split mirrors the backend exactly (`core/memory.rs`, `core/journal.rs`,
// `core/knowledge.rs`, `core/profile.rs`), so a section here maps to one store,
// never to a blend.
//
// Profile is here rather than in Settings, where the old read-only form lived,
// because it is memory: written by watching, decaying with age, and carried into
// every conversation. Filing it under Settings would say it was configuration —
// something you set once — which is exactly the thing it stopped being.

type Section = "memories" | "timeline" | "knowledge" | "profile";

const SECTIONS: { key: Section; label: string; icon: IconName; sub: string }[] = [
  {
    key: "memories",
    label: "Memories",
    icon: "diamond",
    sub: "What your assistants have learned, by source. Everything here is a markdown file on this Mac.",
  },
  {
    key: "timeline",
    label: "Timeline",
    icon: "clock",
    sub: "What was learned and when — daily summaries, the core memory carried into every turn, and the nightly consolidation.",
  },
  {
    key: "knowledge",
    label: "Knowledge",
    icon: "book",
    sub: "What is known about a topic, written deliberately by the agent rather than accumulated.",
  },
  {
    key: "profile",
    label: "Profile",
    icon: "sliders",
    sub: "How you want to be worked with — learned from what you say, and applied to every conversation. Everything here is visible, and yours to pin or remove.",
  },
];

/**
 * Controlled when `section` is supplied (the app drives it from the URL, so Back
 * moves between sections), uncontrolled otherwise (a test, or any caller that just
 * wants the page).
 */
export function BrainView({
  initialSection,
  section: controlled,
  onSectionChange,
}: {
  initialSection?: Section;
  section?: string;
  onSectionChange?: (s: Section) => void;
}) {
  const [internal, setInternal] = useState<Section>(initialSection ?? "memories");
  // An unrecognised `?tab=` is a typo or a stale link, not a reason to render
  // nothing — fall back to the first section rather than a blank page.
  const section = (SECTIONS.some((s) => s.key === controlled) ? (controlled as Section) : null) ?? internal;
  const setSection = (s: Section) => {
    setInternal(s);
    onSectionChange?.(s);
  };
  const current = SECTIONS.find((s) => s.key === section) ?? SECTIONS[0];

  return (
    <main className="flex-1 min-w-0 flex bg-paper">
      <div className="flex-1 min-w-0 overflow-y-auto hairline-scroll">
        <div className="max-w-4xl mx-auto px-7 py-6">
          <PageTabs
            title="Brain"
            tabs={SECTIONS.map((s) => ({ key: s.key, label: s.label, testId: `brain-tab-${s.key}` }))}
            current={section}
            onChange={setSection}
          >
            <PanelHead title={current.label} sub={current.sub} />
            {section === "memories" ? (
              <MemoriesTab />
            ) : section === "timeline" ? (
              <JournalSection />
            ) : section === "profile" ? (
              <ProfileSection />
            ) : (
              <KnowledgeSection />
            )}
          </PageTabs>
        </div>
      </div>
    </main>
  );
}
