// A surface with sections: its name, and the sections as tabs underneath it.
//
// Brain, Settings and Connectors each put their sections in a 208px column of their own,
// between the app's sidebar and the page. That is three columns of navigation before any
// content — and on Brain › Memories, whose content is itself a list beside a reader, four.
// At the 880px window minimum the reader got about fifty pixels and wrapped "Pick a memory to
// read it" one word per line. Connectors spent the whole column on two entries.
//
// The sections now sit as a row of tabs under the page's title, and the width goes back to
// the content. Inbox, Automations and Privacy had three more tab styles of their own (an
// underline under the lead, pills above a title-less page, a boxed segment); they use this
// too, so every surface with sections is one style.
//
// The section's own heading steps back: inside a tabbed page `PanelHead` renders its title
// for screen readers only and keeps the description, because the selected tab already says
// the section's name on screen and saying it twice was the double header this replaces.

import { createContext, useContext, useRef, type ReactNode } from "react";

/** True inside a PageTabs page. Read by PanelHead. */
export const InTabbedPage = createContext(false);

export interface PageTab<K extends string> {
  key: K;
  label: string;
  /** Optional trailing count ("Pending 5"). Null or undefined shows none. */
  count?: number | null;
  /** "warn" when the count is something to go and look at, not just a size. */
  tone?: "warn";
  /** What the section holds, for the tooltip. */
  hint?: string;
  testId?: string;
}

export function PageTabs<K extends string>({
  title,
  tabs,
  current,
  onChange,
  aside,
  children,
}: {
  title: string;
  tabs: PageTab<K>[];
  current: K;
  onChange: (key: K) => void;
  /** Controls that belong to the whole page, on the title's line (a search, an action). */
  aside?: ReactNode;
  children: ReactNode;
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  // Arrow keys move along the row, the way a tab row reads. Enter/Space are the buttons' own.
  const onKey = (e: React.KeyboardEvent, i: number) => {
    const next = e.key === "ArrowRight" ? i + 1 : e.key === "ArrowLeft" ? i - 1 : e.key === "Home" ? 0 : e.key === "End" ? tabs.length - 1 : -2;
    if (next === -2) return;
    e.preventDefault();
    const at = (next + tabs.length) % tabs.length;
    refs.current[at]?.focus();
    onChange(tabs[at].key);
  };
  return (
    <>
      <header className="page-head">
        <div className="page-head-row">
          <h1 className="page-title">{title}</h1>
          {aside}
        </div>
        <nav className="page-tabs" aria-label={`${title} sections`}>
          {tabs.map((t, i) => {
            const on = t.key === current;
            return (
              <button
                key={t.key}
                ref={(el) => {
                  refs.current[i] = el;
                }}
                type="button"
                className={"page-tab" + (on ? " is-on" : "")}
                aria-current={on ? "page" : undefined}
                tabIndex={on ? 0 : -1}
                data-testid={t.testId}
                title={t.hint}
                onClick={() => onChange(t.key)}
                onKeyDown={(e) => onKey(e, i)}
              >
                {t.label}
                {t.count != null && (
                  <span className={"page-tab-count" + (t.tone === "warn" ? " is-warn" : "")}>{t.count}</span>
                )}
              </button>
            );
          })}
        </nav>
      </header>
      <InTabbedPage.Provider value={true}>{children}</InTabbedPage.Provider>
    </>
  );
}

/** Whether the caller is rendering inside a tabbed page. */
export const useInTabbedPage = () => useContext(InTabbedPage);
