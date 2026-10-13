import { AddFolderForm } from "../AddFolderForm";
import { RootRow } from "../RootRow";
import { SEC_H } from "../../ui";
import type { useRoots } from "../../useRoots";

type Roots = ReturnType<typeof useRoots>;

/** Working directories — standing session config (§22/§23 lineage). Flat rows + a quiet "+"
 * link, structurally identical to Sources (owner ask 2026-07-13: the old drawer's card
 * wrapper read too heavy in the rail). */
export function FoldersBlock({
  roots,
  busy,
  error,
  addRoot,
  toggleAccess,
  removeRoot,
  scratchPrimary,
  branch,
  addingFolder,
  setAddingFolder,
}: {
  roots: Roots["roots"];
  busy: Roots["busy"];
  error: Roots["error"];
  addRoot: Roots["addRoot"];
  toggleAccess: Roots["toggleAccess"];
  removeRoot: Roots["removeRoot"];
  scratchPrimary?: boolean;
  branch?: string | null;
  addingFolder: boolean;
  setAddingFolder: (on: boolean) => void;
}) {
  return (
    <div data-testid="drawer-directories">
      <div className={`${SEC_H} mb-1.5`}>Folders</div>
      <div className="-mx-1.5">
        {roots.map((r) => (
          <RootRow
            key={r.path}
            root={r}
            busy={busy}
            scratchPrimary={scratchPrimary}
            branch={r.primary ? branch : undefined}
            onToggle={toggleAccess}
            onRemove={removeRoot}
          />
        ))}
      </div>
      {addingFolder ? (
        <div className="mt-1.5">
          <AddFolderForm
            onAdd={addRoot}
            busy={busy}
            startOpen
            onDismiss={() => setAddingFolder(false)}
          />
        </div>
      ) : (
        <button
          className="mt-1 text-sm text-accent hover:underline text-left"
          onClick={() => setAddingFolder(true)}
        >
          + Give access to a folder…
        </button>
      )}
      {error && <div className="roots-err">{error}</div>}
    </div>
  );
}
