import { SearchModal } from "../components/SearchModal";
import type { AppModel } from "./model/useAppModel";

/** The search modal. */
export function AppSearch({ m }: { m: AppModel }) {
  const { searchOpen, sessions, personas, setSearchOpen, selectSession } = m;
  return (
    <>{searchOpen && (
        <SearchModal
          sessions={sessions}
          personas={personas ?? undefined}
          onSelect={(id, ws, ag) => {
            setSearchOpen(false);
            selectSession(id, ws, ag);
          }}
          onClose={() => setSearchOpen(false)}
        />
      )}</>
  );
}
