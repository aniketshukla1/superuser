import { SearchModal } from "../components/SearchModal";
import type { AppModel } from "./model/useAppModel";

/** The search palette. Picking a session closes it and opens that session. */
export function AppSearch({ m }: { m: AppModel }) {
  if (!m.searchOpen) return null;
  const close = () => m.setSearchOpen(false);
  return (
    <SearchModal
      sessions={m.sessions}
      personas={m.personas ?? undefined}
      onSelect={(id, folder, assistant) => {
        close();
        m.selectSession(id, folder, assistant);
      }}
      onClose={close}
    />
  );
}
