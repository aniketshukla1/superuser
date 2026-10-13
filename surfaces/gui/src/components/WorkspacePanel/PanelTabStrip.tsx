import type { ArtifactInfo } from "../../api";
import { Icon, ICON } from "../Icon";
import type { TabId, panelTabs } from "./Props";

/** The panel's tab strip: the section buttons (with their badges), then open documents. */
export function PanelTabStrip({
  tabs,
  tab,
  onPick,
  touchedCount,
  running,
  browserUnseen,
  docs,
  onSelectDoc,
  onCloseDoc,
}: {
  tabs: ReturnType<typeof panelTabs>;
  tab: TabId;
  onPick: (id: TabId) => void;
  touchedCount: number;
  running?: boolean;
  browserUnseen: boolean;
  docs: ArtifactInfo[];
  onSelectDoc: (path: string) => void;
  onCloseDoc: (path: string) => void;
}) {
  return (
    <div className="wp-tabs" role="tablist">
      {/* The same control the left sidebar uses for Home/Workspace: one recessed
          track, square-ish icon buttons inside it, the current one lit rather than
          underlined. Two panels either side of the same screen were doing the same
          job — switch which section you are looking at — in two different visual
          languages, and the labels were the loudest thing in a strip whose whole
          purpose is to be glanced at.

          The name has not gone anywhere: it is the accessible name and the tooltip,
          which is what a label on a control you use every day should be. */}
      <span className="nav-seg" aria-label="Panel section">
        {tabs.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            aria-label={t.label}
            data-tip={t.label}
            className={"nav-icon-btn wp-tab-btn" + (tab === t.id ? " is-on" : "")}
            onClick={() => onPick(t.id)}
            data-testid={`wp-tab-${t.id}`}
          >
            <Icon name={t.icon} size={ICON.md} />
            {/* The badges move to the corner rather than disappearing with the label.
                A count of touched files and a "something is happening" dot are the
                two things this strip says without being asked, and dropping them to
                save room would be trading the point of the strip for its size. */}
            {t.id === "files" && touchedCount > 0 && (
              <span className="wp-count wp-badge">{touchedCount}</span>
            )}
            {t.id === "activity" && running && <span className="wp-dot wp-badge" aria-label="running" />}
            {t.id === "browser" && browserUnseen && (
              <span className="wp-dot wp-badge" aria-label="a page is waiting" />
            )}
          </button>
        ))}
      </span>

      {docs.length > 0 && <span className="wp-divider" />}
      {docs.map((d) => (
        <button
          key={d.path}
          role="tab"
          aria-selected={tab === `doc:${d.path}`}
          className={"wp-tab doc" + (tab === `doc:${d.path}` ? " active" : "")}
          onClick={() => onSelectDoc(d.path)}
          title={d.path}
        >
          <span className="wp-doc-name">{d.name}</span>
          <span
            className="wp-close"
            role="button"
            aria-label={`Close ${d.name}`}
            onClick={(e) => {
              e.stopPropagation();
              onCloseDoc(d.path);
            }}
          >
            <Icon name="x" size={ICON.sm} />
          </span>
        </button>
      ))}

    </div>
  );
}
