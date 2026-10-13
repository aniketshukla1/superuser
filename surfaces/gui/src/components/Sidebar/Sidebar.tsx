import { EditSidebar } from "../EditSidebar";
import { CreateAssistantModal } from "../CreateAssistantModal";
import { SearchModal } from "../SearchModal";
import { ConfirmDeleteChat } from "../ConfirmDeleteChat";
import type { Props } from "./Props";
import { useSidebarModel } from "./model/useSidebarModel";
import { SidebarBrand } from "./SidebarBrand";
import { SidebarNav } from "./SidebarNav";
import { SidebarList } from "./SidebarList";
import { AccountMenu } from "./AccountMenu";
import { AccountRow } from "./AccountRow";

// Sessions shown per group before "Show more" comes from Settings (sessions_peek, default 5).

export function Sidebar(props: Props) {
  const m = useSidebarModel(props);
  const { editSidebarOpen, navVisible, setNavVisible, setEditSidebarOpen, creatingAssistant,
    setCreatingAssistant, searchModalOpen, personas, setSearchModalOpen, deleting, setDeleting } = m;
  return (
    <div
      className="sidebar flex flex-col min-h-0 bg-paper border-r border-line"
      onMouseLeave={props.onPeekLeave}
    >
      {/* One bar for the whole top: window controls, then where you are.

          It used to be two — an icon row, then a labelled Home/Workspace segmented
          control on its own line below. That second line cost a 24px control plus
          12px of padding out of a column whose whole job is fitting a list, and it
          spent that space restating what two icons say. The icons carry it now,
          sitting at the right end of the same row, and the list starts 36px higher.

          `data-tauri-drag-region` drags the window; on desktop the row clears the
          traffic lights with a left inset rather than by sitting under them. */}
      <SidebarBrand m={m} />

      {/* The app's surfaces, rendered from `nav.ts`. Automations was the only one with a
          row; Brain, Inbox and Connectors were reachable only by opening the account
          menu, which is a poor place for the things you use every day. The menu keeps
          all of them (it renders from the same list), so nothing moved out of reach. */}
      <SidebarNav m={m} />

      {/* No "New assistant" row here. It had one because the action used to be buried
          behind a caret on a big filled button, which nobody would find — but "New" is
          now a row in the group above with its chevron sitting right beside it, so the
          two ended up adjacent, both labelled with a +, doing different things. Two
          plus-rows a few pixels apart cost more than the one they saved.

          The action is unchanged and reachable in one click from that chevron, which is
          also where you pick a persona — the same menu, for the same kind of decision.
          Covered by "reaches the create-assistant flow" in Sidebar.test.tsx, added when
          this row went away and that menu entry became the only way in. */}

      {/* Scroll area: Pinned band + the RECENT header (with group/filter control), then the body —
          grouped (per-persona accordion) or flat (chronological list). */}
      <SidebarList m={m} />

      {/* Bottom (§26): exactly ONE row — the account anchor. The inbox chip on it is
          state-driven with a sticky unlock (quiet when empty, accent + count when pending);
          everything else lives in the account menu, which ALWAYS lists Inbox + Connectors. */}
      <div className="px-2.5 py-2 border-t border-line">
        <div className="relative">
          <AccountMenu m={m} />

          {/* Three controls side by side: the account (opens its menu), the inbox chip, and
              Settings. The chip used to be a span with role=button inside the account
              button — a control inside a control, unreachable by Tab — and Settings could
              only be reached through a menu labelled "Not signed in". */}
          <AccountRow m={m} />
        </div>
      </div>

      {editSidebarOpen && (
        <EditSidebar
          visible={navVisible}
          onChange={setNavVisible}
          onClose={() => setEditSidebarOpen(false)}
        />
      )}

      {creatingAssistant && (
        <CreateAssistantModal
          onClose={() => setCreatingAssistant(false)}
          // Start a session with the assistant you just made — creating it and then
          // hunting for it in a menu is a pointless extra step.
          onCreated={(id) => id && props.onNewSession(id)}
        />
      )}
      {searchModalOpen && (
        <SearchModal
          sessions={props.sessions}
          personas={personas ?? undefined}
          onSelect={(id, ws, ag) => {
            setSearchModalOpen(false);
            props.onSelectSession(id, ws, ag);
          }}
          onClose={() => setSearchModalOpen(false)}
        />
      )}
      {deleting && (
        <ConfirmDeleteChat
          title={deleting.title}
          sessionId={deleting.id}
          messages={deleting.messages}
          onCancel={() => setDeleting(null)}
          onConfirm={() => {
            // Closed first: the row is about to disappear from under the dialog,
            // and a dialog still open over a chat that no longer exists is asking
            // about nothing.
            setDeleting(null);
            props.onDeleteSession(deleting.id);
          }}
        />
      )}
    </div>
  );
}
